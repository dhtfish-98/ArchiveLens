#!/usr/bin/env python3
"""Compare a pinned upstream build with ArchiveLens and compile independent consumers."""
import argparse, hashlib, json, os, pathlib, shutil, struct, subprocess, tempfile, zipfile
P=pathlib.Path
ROOT=P(__file__).resolve().parents[1]
def run(args, **kwargs):
    result=subprocess.run(list(map(str,args)), stdout=subprocess.PIPE, stderr=subprocess.PIPE, **kwargs)
    return result.returncode,result.stdout,result.stderr

def check_execute(args, **kwargs):
    result=run(args,**kwargs)
    if result[0]:raise RuntimeError(f'{args}: {result[2].decode(errors="replace")}')
    return result[1]

def fixture(path):
    header=struct.pack('<8I',0xfeedfacf,0x0100000c,0,2,2,168,0,0)
    segment=struct.pack('<II16sQQQQIIII',0x19,152,b'__TEXT',0x100000000,0x1000,0,267,7,5,1,0)
    section=struct.pack('<16s16sQQIIIIIIII',b'__text',b'__TEXT',0x100000100,8,0x100,2,0,0,0x80000400,0,0,0)
    path.write_bytes((header+segment+section+struct.pack('<4I',0x26,16,264,3)).ljust(0x100,b'\0')+struct.pack('<II',0xd2800540,0xd65f03c0)+b'\x80\x02\x00')

CONSUMER=r'''fn main() {
 let mut sample: u32=0x12345678;
 for index in 0..50000 { sample^=sample<<13;sample^=sample>>17;sample^=sample<<5;
 println!("{:?}",ARM::DECODE(sample,0x100000000+index*4)); }
 for encoding in ["i","@","@\"NSString\"","^i","[4i]","{Point=dd}","(Value=if)","b7","r^v","@?","{Pair=iq}","{Nested={Point=dd}[3i]}","?","v","B","q","[0i]","", "^", "{Bad"] {
 println!("{}",OBJC::MOD::TYPE(encoding)); }
 for (selector,encoding) in [("value","i16@0:8"),("setX:y:","v32@0:8i16d24"),("foo:","@24@0:8@16"),("init", "@16@0:8"),("missing::", "")] {
 println!("{}",OBJC::MOD::SIGN(selector,encoding)); }
}'''

def consumer(work,source,renamed):
    package=work/('consumer-new' if renamed else 'consumer-old');(package/'src').mkdir(parents=True)
    names=['lens-arm64','lens-objc'] if renamed else ['reipa-arm64','reipa-objc']
    dependencies='\n'.join(f'{name} = {{path = {json.dumps(str(source/"crates"/name))}}}' for name in names)
    (package/'Cargo.toml').write_text('[package]\nname="independent-contract"\nversion="0.1.0"\nedition="2021"\n[workspace]\n[dependencies]\n'+dependencies+'\n')
    replacements={'ARM':names[0].replace('-','_'),'OBJC':names[1].replace('-','_'),'DECODE':'lens_decode' if renamed else 'decode','MOD':'lens_type_encoding' if renamed else 'type_encoding','TYPE':'lens_decode_type' if renamed else 'decode_type','SIGN':'lens_method_signature' if renamed else 'method_signature'}
    code=CONSUMER
    for old,new in replacements.items():code=code.replace(old,new)
    (package/'src/main.rs').write_text(code)
    return check_execute(['cargo','run','--quiet','--manifest-path',package/'Cargo.toml'])

def main():
    args=argparse.ArgumentParser();args.add_argument('--upstream',type=P,required=True);args.add_argument('--output',type=P);args.add_argument('--built',action='store_true',help='Use already-built upstream debug and ArchiveLens release binaries')
    opts=args.parse_args();upstream=opts.upstream.resolve()/'reipa';workspace=ROOT/'workspace';report={'project':'ArchiveLens','checks':[],'open':['Interactive GUI launch, Windows execution, real-device IPA/app flows and complete program semantics are unverified.']}
    if not opts.built:
        check_execute(['cargo','build','--locked','--workspace','--manifest-path',upstream/'Cargo.toml'])
        check_execute(['cargo','test','--locked','--workspace','--manifest-path',workspace/'Cargo.toml'])
        check_execute(['cargo','build','--locked','--release','--workspace','--manifest-path',workspace/'Cargo.toml'])
    original=upstream/'target/debug/reipa';rewritten=workspace/'target/release/archivelens'
    def normalize(result):return result[0],result[1].replace(b'ReIPA',b'ArchiveLens').replace(b'reipa',b'archivelens'),result[2].replace(b'ReIPA',b'ArchiveLens').replace(b'reipa',b'archivelens')
    with tempfile.TemporaryDirectory(prefix='archivelens-contract-') as folder:
        work=P(folder);binary=work/'sample.macho';fixture(binary);archive=work/'sample.ipa';bad=work/'bad';bad.write_bytes(b'bad input')
        with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_DEFLATED) as bundle:bundle.writestr('Payload/Sample.app/Sample',binary.read_bytes())
        cases=[[],['--help']]
        for command in ['verify','info','symbols','strings','objc','classdump','swift-types','disasm','decompile']:
            cases.append([command,'--help'])
            for path in [binary,archive,bad,work/'missing']:cases.append([command,str(path)])
        cases += [['disasm',str(binary),'0x100000100','--count','2'],['decompile',str(binary),'0x100000100','--count','2'],['decompile',str(binary),'--all'],['disasm',str(binary),'nonsense'],['disasm',str(binary),'--count','oops']]
        for number,case in enumerate(cases):
            old=normalize(run([original]+case));new=normalize(run([rewritten]+case))
            if old!=new:
                (work/'old.txt').write_bytes(old[1]+old[2]);(work/'new.txt').write_bytes(new[1]+new[2]);raise AssertionError(f'CLI case {number} {case}\noriginal={old}\nrenamed={new}')
            report['checks'].append({'name':f'cli-{number}','status':'PASS','command':list(map(str,case))})
        # Both executables receive the identical destination path; compare every generated file.
        export=work/'export';case=['decompile',str(binary),'--all','--project',str(export)]
        old=normalize(run([original]+case));before={str(p.relative_to(export)):p.read_bytes().replace(b'ReIPA',b'ArchiveLens') for p in export.rglob('*') if p.is_file()}
        shutil.rmtree(export,ignore_errors=True);new=normalize(run([rewritten]+case));after={str(p.relative_to(export)):p.read_bytes().replace(b'ReIPA',b'ArchiveLens') for p in export.rglob('*') if p.is_file()}
        assert old==new and before==after,'project export contract differs'
        report['checks'].append({'name':'project-export-files','status':'PASS','files':len(before)})
        old=consumer(work,upstream,False);new=consumer(work,workspace,True)
        assert old==new,'independent API consumer differs'
        report['checks'].append({'name':'independent-crates','status':'PASS','arm64_words':50000,'type_encodings':20,'method_signatures':5,'output_sha256':hashlib.sha256(new).hexdigest()})
    report['passed']=len(report['checks']);text=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
    if opts.output:opts.output.write_text(text)
    print(text)
if __name__=='__main__':main()
