#!/usr/bin/python

import sys, os, signal, time
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler
from shutil import copyfile

terminateMe = False
counter = 0

'''
class Watcher:

    def __init__(self, from_dir, to_dir):
        self.observer = Observer()
        self.from_dir = from_dir
        self.to_dir = to_dir

    def run(self):
        global terminateMe
        event_handler = Handler()
        self.observer.schedule(event_handler, self.from_dir, recursive=True)
        print self.from_dir
        self.observer.start()
        try:
            while not terminateMe:
                time.sleep(1)
        except:
            self.observer.stop()
            print "Error"

        self.observer.stop()

class Handler(FileSystemEventHandler):

    @staticmethod
    def on_any_event(event):
        global counter, dest
        print "%s -> %s" % (event.src_path, event.event_type)
        if ".cur_input" in event.src_path:
            return None
        if event.is_directory:
            return None
        elif event.event_type in ['created', 'modified']:
            name = "id:{i},src:000000,op:hgf".format(i="%06d" % counter)
            currentDest = os.path.join(dest, name)
            print "Copying file %s to %s" % (event.src_path, currentDest)
            try:
                copyfile(event.src_path, currentDest)
            except:
                print "Copying file failed." 
                return
            counter += 1
'''

def getQueueFiles(folder):
  files = []
  for d in os.listdir(folder):
    p = os.path.abspath(os.path.join(folder, d))
    if os.path.isdir(p):
      files.extend(getQueueFiles(p))
    else:
      basename = os.path.basename(p)
      files.append(p)
  return files

def watcher(source, dest):
  counter = 0
  mem = set()
  while True:
    queue = [f for f in getQueueFiles(source) if f not in mem]
    mem.update(queue)
    print "Found %d new files" % len(queue)
    for f in queue:
      mem.add(f)
      name = "id:{i},src:000000,op:hgf".format(i="%06d" % counter)
      currentDest = os.path.join(dest, name)
      print "Copying file %s to %s" % (f, currentDest)
      try:
          copyfile(f, currentDest)
      except:
          print "Copying file failed." 
          return
      counter += 1
    time.sleep(10)

def signal_handler(sig, frame):
  global terminateMe
  print('You pressed Ctrl+C!')
  terminateMe = True
  sys.exit(0)

signal.signal(signal.SIGINT, signal_handler)

source = sys.argv[1]
dest = os.path.join(sys.argv[2], "queue")

os.system("mkdir -p %s" % dest)

#w = Watcher(source, dest)
#w.run()
watcher(source, dest)