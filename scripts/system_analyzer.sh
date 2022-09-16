
#!/bin/bash

DYLD_LIBRARY_PATH=`pwd`:$DYLD_LIBRARY_PATH target/release/rsllm \
    --query "you are a system analyzer that reports through ai the health of the system and arranges the status in a nice format of the most important fields for the issues seen." \
    --system-prompt "as a system analyzer device with a human soul, Write a poem and base this on the state  of the system, analyze the metrics and talk about the system, print out a formatted set of data points supporting the poem and analysis given. make it humourous" \
    --llm-host http://127.0.0.1:8080  \
    --max-tokens 1000 \
    --daemon \
    --ai-os-stats \
    $@