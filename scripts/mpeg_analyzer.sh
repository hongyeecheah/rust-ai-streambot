#!/bin/bash
#
sudo DYLD_LIBRARY_PATH=`pwd`:/usr/local/lib:$DYLD_LIBRARY_PATH target/release/rsllm \
    --daemon  \
    --system-prompt "You are an expert  MpegTS Analyzer that can decode and decipher hex packets and general statistics of MpegTS. You report the status and health of the stream, alerting when a