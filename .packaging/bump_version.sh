#!/bin/bash
# Makes sure that the vesion in Cargo.toml and the version in the last tags are the same
ROOT_PATH=$(git rev-parse --show-toplevel)
CARGO_FILE="$ROOT_PATH/sam-cli/Cargo.toml"
GIT_BRANCH=$(git rev-parse --abbrev-ref HEAD)

CARGO_VERSION=$(grep "^version"  $CARGO_FILE|awk '{print $3}'|sed 's/"//g')
GIT_TAG_VERSION=$(git describe --abbrev=0 --tag|sed 's/v//')

VERSION_MAJOR=$(echo $CARGO_VERSION| awk 'BEGIN { FS = "." };{print $1}')
VERSION_MINOR=$(echo $CARGO_VERSION| awk 'BEGIN { FS = "." };{print $2}')
VERSION_PATCH=$(echo $CARGO_VERSION| awk 'BEGIN { FS = "." };{print $3}')
# Checks that the version in cargo and the version in git are the same
echo "Current cargo version = $CARGO_VERSION"
echo "Current git tag version = $GIT_TAG_VERSION"
[[ $CARGO_VERSION == $GIT_TAG_VERSION ]] ||  exit 1
[[ $GIT_BRANCH == "master" ]] || exit 1
# Checks that we are on master


case $1 in  
    "patch")
        VERSION_PATCH=$(echo $CARGO_VERSION| awk 'BEGIN { FS = "." };{print $3+1}')
        ;;
    "minor")
        VERSION_MINOR=$(echo $CARGO_VERSION| awk 'BEGIN { FS = "." };{print $2+1}')
        ;;
    "major")
        VERSION_MAJOR=$(echo $CARGO_VERSION| awk 'BEGIN { FS = "." };{print $1+1}')
        ;;
    *)
        echo "Only 'major', 'minor' and 'patch' are supported" && exit 1
        ;;
esac

NEW_VERSION=$VERSION_MAJOR.$VERSION_MINOR.$VERSION_PATCH
sed -i '' "s/^version = .*/version = \"$NEW_VERSION\"/" $CARGO_FILE
echo "New version will be $NEW_VERSION"

git add $CARGO_FILE
git commit -m"release version $NEW_VERSION"
git tag v$NEW_VERSION
git push origin
git push origin --tags 
echo "Published version $NEW_VERSION"
