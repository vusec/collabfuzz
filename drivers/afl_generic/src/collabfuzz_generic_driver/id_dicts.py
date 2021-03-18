from typing import NamedTuple, Dict


class IDDicts(NamedTuple):
    local_to_server_ids: Dict[int, str] = {}
    sync_to_server_ids: Dict[int, str] = {}
