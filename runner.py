import subprocess
import datetime
import time
import logging
import re
import sys
import os
import signal

MAX_TIMEOUT = 6 * 60 * 60
logger = logging.getLogger(__name__)
stdout_handler = logging.StreamHandler(sys.stdout)
file_handler = logging.FileHandler(f'runner_{datetime.datetime.now()}.log')

logger.addHandler(stdout_handler)
logger.addHandler(file_handler)
logger.setLevel(logging.DEBUG)

def get_res(line):
    m_jobs = re.search(r"#(\d+)\s*(\w+)\s*cov: (?P<cov>\d+).*", line)
    m_fork = re.search(r"#(?P<num>\d+): cov: (?P<cov>\d+) ft: (?P<ft>\d+) corp: (?P<corp>\d+) exec/s (?P<exec>\d+) oom/timeout/crash: (?P<oom>\d+)/(?P<to>\d+)/(?P<crash>\d+).*", line)
    if m_jobs:
        return m_jobs.groupdict()
    elif m_fork:
        return m_fork.groupdict()
    return None

# run a new process with timeout
def run_with_timeout(cmd, timeout):
    start_time = datetime.datetime.now()
    is_jobs = True if "-jobs" in cmd else False
    proc = subprocess.Popen(cmd, shell=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    outs = []

    off = 0
    old_cov = 0
    while proc.poll() == None:
        # time.sleep(30)
        if is_jobs:
            with open("./fuzz-0.log", "r") as f:
                f.seek(off)
                data = f.read()
                off += len(data)
        else:
            data = proc.stderr.read1().decode("utf-8")
        # print(data)
        for line in data.split("\n")[::-1]:
            res = get_res(line)
            if res:
                cov = int(res["cov"])
                changed = (cov - old_cov) / cov
                if changed >= 0.01:
                    logger.info(f"{datetime.datetime.now()}: [+] changed: {changed} {line}")
                else:
                    logger.info(f"{datetime.datetime.now()}: [-] changed: {changed} {line}")
                old_cov = cov
                break
        for line in data.split("\n"):
            if len(outs) >= 300:
                outs.pop(0)
            outs.append(line)

        if datetime.datetime.now() - start_time > datetime.timedelta(seconds=timeout):
            os.killpg(os.getpgid(proc.pid), signal.SIGTERM)  # Send the signal to all the process groups
            proc.terminate()
            proc.communicate()

    return outs

def main():
    # get current time
    current_datetime = datetime.datetime.now()
    # run a new process with timeout. If the process ends before the timeout, rerun it to the end of the timeout

    while datetime.datetime.now() - current_datetime < datetime.timedelta(hours=24):
        gap = (datetime.datetime.now() - current_datetime)
        outs = run_with_timeout("./run-fuzzer.sh -fork=30 -rss_limit_mb=10240", MAX_TIMEOUT - gap.seconds)
        with open(f"output_{datetime.datetime.now()}.log", "w") as f:
            f.write("\n".join(outs))

main()
