#!/bin/bash
#

DYLD_LIBRARY_PATH=`pwd`:$DYLD_LIBRARY_PATH target/release/rsllm \
    --daemon \
    --query "Determine if the system is healthy or sick, diagnose the issue if possible or give details about it. Use the historical view to see bigger trends of the system. draw a table of the current system metr