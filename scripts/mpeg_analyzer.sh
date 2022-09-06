#!/bin/bash
#
sudo DYLD_LIBRARY_PATH=`pwd`:/usr/local/lib:$DYLD_LIBRARY_PATH target/release/rsllm \
    --daemon  \
    --system-prompt "