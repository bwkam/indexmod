# get the version
# https://github.com/ltfschoen/kobold-test/blob/master/set_cargo_package_version.sh
LINE_START=$(grep -n -m 1 "\[package\]" Cargo.toml | cut -f1 -d:)
echo "Found [package] on line: $LINE_START"
LINE_VERSION=$(awk "NR >= $LINE_START && /version/{print NR}" Cargo.toml | head -1)
echo "Found [package] version on line: $LINE_VERSION"
LINE_VERSION_CONTENTS=$(awk "NR==$LINE_VERSION{ print; exit }" Cargo.toml)
echo "Contents of [package] version line number: $LINE_VERSION_CONTENTS"
CARGO_PACKAGE_VERSION=$(echo "$LINE_VERSION_CONTENTS" | sed 's/version//;s/=//;s/\"//g' | xargs)
echo "Package [package] version number is: $CARGO_PACKAGE_VERSION"
# remove old zips
rm -rf *.zip
# zip new project
zip -r "cell-reply-app-$CARGO_PACKAGE_VERSION.zip" src assets templates Cargo.toml Cargo.lock tests
