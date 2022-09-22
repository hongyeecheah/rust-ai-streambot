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
#MODEL=g