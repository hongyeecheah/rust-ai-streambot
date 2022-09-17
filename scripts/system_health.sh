#!/bin/bash
#

DYLD_LIBRARY_PATH=`pwd`:$DYLD_LIBRARY_PATH target/release/rsllm \
    --daemon \
    --query "Determine if the system is healthy or