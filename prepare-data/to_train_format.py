#! /usr/bin/env python
import glob

import os
import sys
import json

if __name__ == '__main__':
    source_dir = sys.argv[1]
    target_dir = sys.argv[2]
    
    if not os.path.exists(target_dir):
        os.makedirs(target_dir)
        
    # Preparing the data set for fine-tuning
    files = glob.glob(source_dir + '/*/**.txt', recursive=True)
    contents = []
    for file in files:
        with open(file, 'r') as f:
            content = f.read()
            contents.append({"text": content})
            f.close()
        if len(contents) > 1000:
            with open(target_dir + '/train.json', 'a') as f:
                f.write(json.dumps(contents))
                f.close()
            contents.clear()
                