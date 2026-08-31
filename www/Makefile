DESTDIR  ?=
WWWDIR   := $(DESTDIR)/var/httpd/www

.PHONY: install clean

preview:
	open http://localhost:8080/cgi-bin/contelia.hsl
	busybox httpd -f -p8080 -h ./

install:
	install -D       cgi-bin/contelia.hsl $(WWWDIR)/cgi-bin/contelia
	install -D -m644 style.css            $(WWWDIR)/style.css
	install -D -m644 header.js            $(WWWDIR)/header.js
	install -D -m644 body.js              $(WWWDIR)/body.js
	install -D -m644 contelia-256.png     $(WWWDIR)/contelia-256.png

clean:
	@true
