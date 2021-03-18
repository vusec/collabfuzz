#!/usr/bin/python

import os, sys, signal, time, random, threading
from shutil import copyfile

terminated = False
def getQueueFiles(folder):
  files = []
  for d in os.listdir(folder):
    p = os.path.abspath(os.path.join(folder, d))
    if os.path.isdir(p):
      files.extend(getQueueFiles(p))
    else:
      files.append(p)
  return files

def signal_handler(sig, frame):
  global terminated
  terminated = True
  print('You pressed Ctrl+C!')
  sys.exit(0)

def sync_thread():
  global terminated, corpusDir, randID 
  alreadyCopied = set()
  counter = 0
  while not terminated:
    files = getQueueFiles(corpusDir)
    for f in files:
      if f not in alreadyCopied:
        s = "/dev/shm/sync/libfuzzer{randID}/queue/id:{counter},src:000000,op:lbf"
        s = s.format(randID=randID, counter=counter)
        print "Copying %s" % s
        copyfile(f, s)
        alreadyCopied.add(f)
        counter += 1
    time.sleep(1)

signal.signal(signal.SIGINT, signal_handler)

print "running libfuzzer"

waitingPeriod = 1*60 #10min
scriptPath = os.path.realpath(__file__)
projectPath = os.path.abspath(os.path.dirname(scriptPath))

randID = os.environ['RAND_ID']
binaryPath = os.environ['PROJECT_BINARY_PATH']
syncName = os.environ['SYNC_NAME']
corpusDir = "/dev/shm/libfuzzer{randID}".format(randID=randID)

print "scriptPath: %s\nprojectPath: %s\nrandID: %s" % (scriptPath, projectPath, randID)
print binaryPath

os.system("rm -rf /dev/shm/sync/libfuzzer{randid}; mkdir -p /dev/shm/sync/libfuzzer{randid}/queue/; mkdir -p /dev/shm/libfuzzer{randid}".format(randid=randID))

t = threading.Thread(target=sync_thread)
t.start()

while not terminated:
  os.system(binaryPath)
  print "Libfuzzer execution terminated, probably a crash was found, restarting"