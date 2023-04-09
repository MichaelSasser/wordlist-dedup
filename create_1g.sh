LC_ALL=C tr '\0-\377' '[0*25][1*25][2*25][3*25][4*25][5*25][6*25][7*25][8*25][9*25][x*]' < /dev/urandom |
 tr -d x |
 fold -w 1 |
 paste -sd "$(printf '%99s\\n')" - |
 /usr/bin/head -c1G > /tmp/sample.txt
