from fabric import SerialGroup, Group, Connection, task, ThreadingGroup
from typing import List
import yaml, getpass, os

SETTINGS_FILE = "settings.yaml"
WORKING_ROOT = "~/Projects"
WORKING_DIR = "~/Projects/actix-telepathy/"
VENV_NAME = "dfl"
VENV_SOURCE_PATH = os.path.join(WORKING_DIR, VENV_NAME, "bin/activate")
PYTHON_PATH = os.path.join(WORKING_DIR, VENV_NAME, "bin/python")


def load_settings() -> dict:
    with open(SETTINGS_FILE, "r") as f:
        settings = yaml.load(f, yaml.Loader)
    return settings


def host2hoststr(host: dict) -> str:
    return f"{host['user']}@{host['hostname']}"


def get_group_connection(gateway: Connection) -> Group:
    settings = load_settings()
    hosts = settings["fabric"]["hosts"]
    password = getpass.getpass(f"{hosts[0]['user']}@{hosts[0]['hostname']}'s password: ")
    c = SerialGroup(*[host2hoststr(h) for h in hosts], gateway=gateway,
                    connect_kwargs={"password": password})
    return c


def get_single_connection(gateway: Connection, hostname: str) -> Connection:
    settings = load_settings()
    host = list(filter(lambda x: x['hostname'] == hostname, settings["fabric"]["hosts"]))[0]
    password = getpass.getpass(f"{host2hoststr(host)}'s password: ")
    c = Connection(host2hoststr(host), gateway=gateway, connect_kwargs={"password": password})
    return c


def get_single_connections(gateway: Connection) -> List[Connection]:
    settings = load_settings()
    hosts = list(filter(lambda x: x['hostname'] == hostname, settings["fabric"]["hosts"]))
    connections = []
    for host in hosts:
        password = getpass.getpass(f"{host2hoststr(host)}'s password: ")
        connections.append(Connection(host2hoststr(host), gateway=gateway, connect_kwargs={"password": password}))
    return connections


@task
def hostname(gateway):
    c = get_group_connection(gateway)
    c.run("hostname", pty=True)


@task
def install(gateway):
    c = get_group_connection(gateway)
    c.run(f"mkdir -p {WORKING_ROOT}; cd {WORKING_ROOT}; git clone git@github.com:wenig/actix-telepathy.git")
    c.run(f"cd {WORKING_DIR}; cargo build --workspace")
    c.run(f"cd {WORKING_DIR}; cargo test --package decentfl")


@task
def update(gateway):
    c = get_group_connection(gateway)
    c.run(f"cd {WORKING_DIR}; git pull")
    c.run(f"cd {WORKING_DIR}; cargo build --workspace")
    c.run(f"cd {WORKING_DIR}; cargo test --package decentfl")


@task
def basic_test(gateway):
    c = get_group_connection(gateway)
    c.run(f"screen -dm bash -c 'cd {WORKING_DIR}; bash start_basic.sh'")


@task
def experiment(gateway):
    c = get_group_connection(gateway)
    settings = load_settings()["fabric"]["experiment"]
    base_port: int = settings["settings"]["base_port"]
    kwargs = to_terminal_args(settings["kwargs"])
    c.run(f"screen -dm bash -c 'cd {WORKING_DIR}; DECENTFL_BASEPORT={base_port} DECENTFL_BASEHOST=odin01 bash start_decentfl.sh {kwargs}'")


def to_terminal_args(kwargs: dict) -> str:
    res = ""
    for k, v in kwargs.items():
        res += f"--{k} {v} "
    return res


@task
def check_for_running(gateway):
    c = get_single_connection(gateway, "odin01")
    c.run(f"screen -ls")


@task
def download_results(gateway, gather=True):
    c = get_single_connection(gateway, "odin01")
    if gather:
        c.run(f"cd {WORKING_DIR}; ./scp_all.sh")
    files = c.run(f"ls {os.path.join(WORKING_DIR, 'decentfl*.db')}", hide="out")
    files = files.stdout.strip().split("\n")
    for file in files:
        print(f"download {file}")
        c.get(os.path.join(file))
        local_file = file.split("/")[-1]
        os.replace(local_file, f"results/{local_file}")
