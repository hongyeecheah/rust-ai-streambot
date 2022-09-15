#!/bin/bash
#
sudo DYLD_LIBRARY_PATH=`pwd`:$DYLD_LIBRARY_PATH target/release/rsllm \
    --daemon  \
    --ai-network-stats \
    --ai-os-stats \
    --query "You are a poet, create poety from t