import subprocess
import datetime
import time

# run a new process with timeout
def run_with_timeout(cmd, timeout):
    proc = subprocess.Popen(cmd, shell=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        outs, errs = proc.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        proc.kill()
        outs, errs = proc.communicate()
    return outs, errs

def main():
    # get current time
    current_datetime = datetime.datetime.now()
    # run a new process with timeout. If the process ends before the timeout, rerun it to the end of the timeout

    MAX_TIMEOUT = 24 * 60 * 60


    while datetime.datetime.now() - current_datetime < datetime.timedelta(hours=24):
        gap = (datetime.datetime.now() - current_datetime)
        outs, errs = run_with_timeout("./run-fuzzer.sh", MAX_TIMEOUT - gap.seconds)
        with open(f"output_{datetime.datetime.now()}.log", "w") as f:
            f.write("\n".join(outs.decode("utf-8").split("\n")[-300:]) + "\n\n\n")
            f.write(errs.decode("utf-8"))

main()
