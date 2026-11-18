"""Content-addressed executable storage for resumable VIES collection."""
import fcntl
import shutil
from vies_evidence import sha


def freeze_binary(source, cache):
    """Keep one immutable executable per digest, including after collection ends."""
    digest = sha(source)
    cache.mkdir(parents=True, exist_ok=True)
    with (cache / '.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        frozen = cache / digest
        if not frozen.exists():
            temporary = frozen.with_suffix('.tmp')
            shutil.copyfile(source, temporary)
            if sha(temporary) != digest:
                temporary.unlink()
                raise ValueError('source binary changed while freezing')
            temporary.chmod(0o500)
            temporary.replace(frozen)
        if sha(frozen) != digest:
            raise ValueError('cached frozen binary changed')
    return frozen
