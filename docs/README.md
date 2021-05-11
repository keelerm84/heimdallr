> :warning: This project is intended as a playground to explore rust. **Long
> term maintenance is not guaranteed.** Buyer beware.

[![Trust but verify](https://github.com/keelerm84/heimdallr/actions/workflows/trust-but-verify.yml/badge.svg)](https://github.com/keelerm84/deploy/actions/workflows/trust-but-verify.yml)

# heimdallr

Connect to AWS EC2 hosts via a Bastion / Jump host

## Configuration file

You must create a configuration file located at `~/.config/heimdallr.toml`. An
example configuration is shown below.

```toml
[profiles]

[profiles.default]
aws_profile = "default"
security_group_id = "sg-12345678"
dns_name = "bastion.example.io"
bastion_port = 1234
bastion_user = "example-user"
ec2_user = "ec2-user"
identity_file = "~/.ssh/id_rsa"
```

Note that each of these options can be overridden with an equivalent command
line option. This allows you to define reasonable defaults, but the flexible to
override when needed.

## Usage and examples

```console
$ heimdallr --help

heimdallr 0.1.0
Connect to AWS EC2 hosts via a Bastion / Jump host

USAGE:
    heimdallr [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -p, --profile <profile>    Profile name as specified in your configuration file [default: default]

SUBCOMMANDS:
    connect    Connect to a running instance
    grant      Add your IP to a security group to allow ingress
    help       Prints this message or the help of the given subcommand(s)
    list       List all running instances
    revoke     Revoke your IP from a security group to prevent future ingress
    update     Update this executable to the latest version
```

List instances available to connect to.

```console
$ heimdallr list
│ Environment │ Name                  │ Instance Id         │
├─────────────┼───────────────────────┼─────────────────────┤
│ Production  │ ProductionInstance1   │ i-12345678901234567 │
│ Staging     │ StagingInstance1      │ i-12345678901234567 │
```

Add your IPv4 address to the specified security group (with optional description).

```console
$ heimdallr --profile default grant --description "Home machine"
```

Remove your IPv4 address from the specified security group.

```console
$ heimdallr --profile default revoke
```

Generate the appropriate ssh command to:

**Connect to an EC2 instance**

```console
$ heimdallr --profile default connect StagingInstance1
ssh -i ~/.ssh/id_rsa -p 1234 -A -t example-user@bastion.example.io ssh -A -t ec2-user@PRIVATE-IP bash
```

**Connect to a service running on a specific cluster.**

```console
$ heimdallr --profile default connect cluster#service
ssh -i ~/.ssh/id_rsa -p 1234 -A -t example-user@bastion.example.io "ssh -A -t ec2-user@PRIVATE-IP \"docker exec -it -detach-keys 'ctrl-q,q' SERVICE_CONTAINER_RUNTIME_ID bash\""
```

**Connect to a service running on a specific cluster while override configuration options.**

```console
$ heimdallr --profile default connect --dns-name bastion-staging.example.io --bastion-user bastion-user cluster#service
ssh -i ~/.ssh/id_rsa -p 1234 -A -t bastion-user@bastion-staging.example.io "ssh -A -t ec2-user@PRIVATE-IP \"docker exec -it -detach-keys 'ctrl-q,q' SERVICE_CONTAINER_RUNTIME_ID bash\""
```

**Connect to a particular container if the service is running multiple tasks**

```console
$ heimdallr --profile default connect cluster#service#container
ssh -i ~/.ssh/id_rsa -p 1234 -A -t example-user@bastion.example.io "ssh -A -t ec2-user@PRIVATE-IP \"docker exec -it -detach-keys 'ctrl-q,q' SERVICE_CONTAINER_RUNTIME_ID bash\""
```

**Connect and run arbitrary command**

```console
$ heimdallr --profile default connect cluster#service#container ls -lah
ssh -i ~/.ssh/id_rsa -p 1234 -A -t example-user@bastion.example.io "ssh -A -t ec2-user@PRIVATE-IP \"docker exec -it -detach-keys 'ctrl-q,q' SERVICE_CONTAINER_RUNTIME_ID ls -lah\""
```

## Release process

Install [cargo-make][cargo-make] and run the following command on main.

```console
cargo make release
```

This will ensure that the project is in good shape (`cargo test`, `cargo
clippy`, `cargo build`, etc), generate a changelog and bump the appropriate
versions.

Once the commit and tag is pushed, a GitHub action will run to build static
binaries and associate those artifacts with the latest release.

## FAQ

### Why doesn't the list command return any results?

This code assumes you are making use of tags on your ec2 instances. Be sure to
set Name and Env tags on each instance.

## License

[MIT](./LICENSE.md)

## Acknowledgment

This project is a Rust re-implementation from an existing bash project. You can view
the original project [here][heimdall].

[heimdall]: https://github.com/needcaffeine/heimdall
[cargo-make]: https://github.com/sagiegurari/cargo-make
