import glob
head=open('src/shell_head.html').read(); tail=open('src/shell_tail.html').read()
js=''.join(open(f).read() for f in sorted(glob.glob('src/[0-9]*.js')))
out=head+'<script>\n'+js+tail
j=out.index('</style>')+8
open('arena-cero.html','w').write('<!doctype html>\n<html lang="es">\n<head>\n<meta charset="utf-8">\n<meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">\n'+out[:j]+'\n</head>\n<body>'+out[j:]+'\n</body>\n</html>\n')
