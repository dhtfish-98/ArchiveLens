#!/usr/bin/env python3
"""Use the same resolved third-party versions for the upstream comparison."""
import pathlib,re,sys
root=pathlib.Path(__file__).resolve().parents[1]
text=(root/'workspace/Cargo.lock').read_text()
for new,old in [('archivelens','reipa'),('lens-macho','reipa-macho'),('lens-image','reipa-image'),('lens-objc','reipa-objc'),('lens-swift','reipa-swift'),('lens-arm64','reipa-arm64'),('lens-bench','reipa-bench'),('lens-gui','reipa-gui')]:text=text.replace('"'+new+'"','"'+old+'"')
(pathlib.Path(sys.argv[1])/'reipa/Cargo.lock').write_text(text)
