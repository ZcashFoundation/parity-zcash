#!/bin/bash

set -e # fail on any error
set -u # treat unset variables as error
rm -rf deb
#create DEBIAN files
mkdir -p deb/usr/bin/
mkdir -p deb/DEBIAN
#create copyright, docs, compat
cp LICENSE deb/DEBIAN/copyright
echo "https://github.com/ZcashFoundation/zebra/" >> deb/DEBIAN/docs
echo "8" >> deb/DEBIAN/compat
#create control file
control=deb/DEBIAN/control
echo "Package: zebra" >> $control
version=`grep -m 1 version Cargo.toml | awk '{print $3}' | tr -d '"' | tr -d "\n"`
echo "Version: $version" >> $control
echo "Source: zebra" >> $control
echo "Section: science" >> $control
echo "Priority: extra" >> $control
echo "Maintainer: Zcash Foundation" >> $control
echo "Build-Depends: debhelper (>=9)" >> $control
echo "Standards-Version: 3.9.5" >> $control
echo "Homepage: https://zfnd.org" >> $control
echo "Vcs-Git: git://github.com/ZcashFoundation/zebra.git" >> $control
echo "Vcs-Browser: https://github.com/ZcashFoundation/zebra" >> $control
echo "Architecture: $1" >> $control
echo "Depends: libssl1.0.0 (>=1.0.0)" >> $control
echo "Description: Zcash node client" >> $control
#build .deb package

exit
