#!/usr/bin/env python3
import os
import subprocess
from hashlib import md5

# Configuration
music_dir = "/Volumes/BrahmaSSD/music/AiGen"
output_file = "/tmp/combined_playlist.wav"
playlist_file = "/tmp/ffmpeg_playlist.txt"
checksum_file = "/tmp/playlist_checksum.txt"

def get_files_sorted_by_mtime(directory, extension=".wav"):
    files