#!/bin/bash
#
# Alice's AI Wonderland Character:
# - Parody of walt-disney's original Alice animations, the first ones that got published.
#
# RsLLM configuration script:
# - @2024 Christi Kennedy - This is not related to any known alices or wonderlands.
#
#

# === CONFIGURATION ===
BUILD_TYPE=release
## Interstitial message
GREETING="Hi I'm Alice, ask me a question by typing '!message Alice <message>' or chat with me in the chat. Please remember to follow me!"
## LLM Model Config
# Candle settings
USE_CANDLE=0
MODEL=mistral
#MODEL=gemma
MODEL_ID=7b-it
# Generic settings
USE_API=1
#CHAT_FORMAT=chatml
#CHAT_FORMAT=llama2
CHAT_FORMAT=vicuna
MAX_TOKENS=800
TEMPERATURE=0.8
CONTEXT_SIZE=8000
QUANTIZED=0
KEEP_HISTORY=1
SD_MAX_LENGTH=50
## Pipeline Settings
DAEMON=1
CONTINUOUS=0
POLL_INTERVAL=60000
PIPELINE_CONCURRENCY=6
ASYNC_CONCURRENCY=0
NDI_