from .connection import FrameworkConnection, ConnectionException
from .watcher import WatcherException, get_watcher
from .receiver import ReceiverException, get_receiver
from .cmdline import get_cmdline_suggestion
from .id_dicts import IDDicts
from .config import Config

from contextlib import closing
import time
import sys

CRASH_POLL = 1  # Polling interval to check if the threads crashed in seconds


def start_driver(config: Config):
    try:
        print("Attempting to contact server...")
        connection = FrameworkConnection(config)  # Blocking call
    except KeyboardInterrupt:
        print("Program aborted while waiting for server")
        exit(1)
    except ConnectionException as e:
        print(f"Registration with server failed failed: {e}")
        exit(1)

    with closing(connection):
        try:
            id_dicts = IDDicts()
            watcher = get_watcher(config, connection, id_dicts)
            receiver = get_receiver(config, connection, id_dicts)

            receiver.start()

            cmdline_suggestion = get_cmdline_suggestion(config)
            print(f'Start your fuzzer using: "{cmdline_suggestion}"')

            print(f"Waiting for fuzzer...")
            watcher.start()  # Blocking call
        except (WatcherException, ReceiverException) as e:
            print(f"Error while instantiating components: {e}")
            exit(1)

        try:
            # Wait until a keyboard interrupt arrives.
            print(f"The driver is running, exit with ^C.", flush=True)
            while True:
                # Crash this thread if receiver or watcher crashed
                if not watcher.is_alive():
                    print(
                        "The watcher crashed, killing the remaining components",
                        file=sys.stderr,
                    )
                    raise WatcherException("Watcher crashed!")
                elif not receiver.is_alive():
                    print(
                        "The receiver crashed, killing the remaining components",
                        file=sys.stderr,
                    )
                    raise ReceiverException("Receiver crashed!")

                time.sleep(CRASH_POLL)

        except KeyboardInterrupt:
            # Exit normally
            watcher.stop()
            receiver.stop()

        except (WatcherException, ReceiverException):
            # Exit abruptly
            watcher.stop()
            receiver.stop()
            exit(1)
