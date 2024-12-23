@REM NOT WORK: SVML LIB cat not call
set fname=freq_modulation.c
set inter_out=freq_modulation.o
set out=freq_modulation.lib

set build_options=/fast /Oi /fp:fast /fp:except- /Qimf-use-svml:true /Qintrinsic-promote /Qvec-peel-loops /Qvec-remainder-loops
icx -c %build_options% -o %inter_out% %fname%
@REM && lib   /nologo /out:%out% %inter_out%