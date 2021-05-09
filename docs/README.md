> :warning: This project is intended as a playground to explore rust. **Long
> term maintenance is not guaranteed.** Buyer beware.

[![Trust but verify](https://github.com/keelerm84/heimdallr/actions/workflows/trust-but-verify.yml/badge.svg)](https://github.com/keelerm84/deploy/actions/workflows/trust-but-verify.yml)

# heimdallr

Connect to AWS EC2 hosts via a Bastion / Jump host

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
    -p, --profile <profile>    Profile name as specified in your configuration file

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
$ heimdallr grant --security-group-id sg-12345678 --description "Home machine"
```

Remove your IPv4 address from the specified security group.

```console
$ heimdallr revoke --security-group-id sg-12345678
```

Generate the appropriate ssh command to:

**Connect to an EC2 instance**

```console
$ heimdallr connect --dns-name bastion.example.io --bastion-port 1234 --bastion-user example-user --ec2-user ec2-user --identity-file ~/.ssh/id_rsa StagingInstance1
ssh -i ~/.ssh/id_rsa -p 1234 -A -t example-user@bastion.example.io ssh -A -t ec2-user@PRIVATE-IP bash
```

**Connect to a service running on a specific cluster.**

```console
$ heimdallr connect --dns-name bastion.example.io --bastion-port 1234 --bastion-user example-user --ec2-user ec2-user --identity-file ~/.ssh/id_rsa cluster#service
ssh -i ~/.ssh/id_rsa -p 1234 -A -t example-user@bastion.example.io "ssh -A -t ec2-user@PRIVATE-IP \"docker exec -it -detach-keys 'ctrl-q,q' SERVICE_CONTAINER_RUNTIME_ID bash\""
```

**Connect to a particular container if the service is running multiple tasks**

```console
$ heimdallr connect --dns-name bastion.example.io --bastion-port 1234 --bastion-user example-user --ec2-user ec2-user --identity-file ~/.ssh/id_rsa cluster#service#container
ssh -i ~/.ssh/id_rsa -p 1234 -A -t example-user@bastion.example.io "ssh -A -t ec2-user@PRIVATE-IP \"docker exec -it -detach-keys 'ctrl-q,q' SERVICE_CONTAINER_RUNTIME_ID bash\""
```

**Connect and run arbitrary command**

```console
$ heimdallr connect --dns-name bastion.example.io --bastion-port 1234 --bastion-user example-user --ec2-user ec2-user --identity-file ~/.ssh/id_rsa cluster#service#container ls -lah
ssh -i ~/.ssh/id_rsa -p 1234 -A -t example-user@bastion.example.io "ssh -A -t ec2-user@PRIVATE-IP \"docker exec -it -detach-keys 'ctrl-q,q' SERVICE_CONTAINER_RUNTIME_ID ls -lah\""
```

## FAQ

### Why doesn't the list command return any results?

This code assumes you are making use of tags on your ec2 instances. Be sure to
set Name and Env tags on each instance.

## License

[MIT](./LICENSE.md)

## Roadmap

- Add GitHub actions to build static binaries and publish to release page
- Complete `update` subcommand to allow in place updates of executable
- Introduce configuration file to reduce command line parameter requirements
- Lots of refactoring as I learn more about rust :crab:

## Acknowledgment

This project is a Rust re-implementation from an existing bash project. You can view
the original project [here][heimdall].

[heimdall]: https://github.com/needcaffeine/heimdall
