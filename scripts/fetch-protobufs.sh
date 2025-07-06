#!/bin/bash

set -euo pipefail

git_root="$(git rev-parse --show-toplevel)"
destinationDir="${git_root}/vendor/github.com/SteamDatabase/Protobufs"

# SteamDB protobuf commit that we want to base these files off
protobufCommit="031ff24abb31e19e9119393807ab594608b214c0"

# Create temp folder for checking out the SteamDB protobuf repo
temp=$(mktemp -d)
cd "${temp}"

# Sparsely checkout the repo with just the folders that we care about
git clone https://github.com/SteamDatabase/Protobufs.git --no-checkout .
git sparse-checkout set dota2 google
git checkout "${protobufCommit}"

# These files cause the protobuf build to break and we dont need them
rm dota2/steammessages_base.proto dota2/steammessages_clientserver_login.proto

# Append header to the protobuf files as they compile with a modern compiler
for file in dota2/*.proto; do
  mv "${file}" "${file}.orig"
  cat << EOF >> "${file}"
syntax = "proto2";

EOF

  cat "${file}.orig" >> "${file}"
  rm "${file}.orig"
done

# Copy files back over to this repo
mkdir -p ${destinationDir}
rm -rf ${destinationDir}/dota2
cp -r dota2 ${destinationDir}/dota2
cp -r google ${destinationDir}/dota2/google

# Remove temp folder
rm -rf "{$temp}"

# Celebrate
echo "Successfully generated protobuf files 🎉"
