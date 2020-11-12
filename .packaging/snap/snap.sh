#!/bin/sh
cd `git rev-parse --show-toplevel`
cd .packaging/snap

mkdir .snapcraft
echo $SNAP_LOGIN_FILE | base64 --decode --ignore-garbage > .snapcraft/snapcraft.cfg
snapcraft
snapcraft push *.snap --release edge