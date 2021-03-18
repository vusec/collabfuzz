

collect all bc files from binutils:
`find . -type f -executable ! -name "*.*" ! -name "Makefile" -exec sh -c "file {} ; extract-bc {} && cp {}.bc ." \;`
