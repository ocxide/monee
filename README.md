# monee

Monee is a suite of tools for registering all your finances locally. 
Never forget your expenses!.

Monee will be avaliable in various platforms so you can use it everywhere.

## CLI

Read your finances and add new events via the command line.

### Installation

```sh
chmod +x ./monee-cli/deploy.sh
./monee-cli/deploy.sh
```

run --help to see the available commands

```sh
monee --help
```

## Features

You can register:

- currencies: the currency of your expenses
- actors: individuals or organizations you work with
- wallets: your current balances
- in-debts: debts from others to you
- out-debts: debts from you to others

monee will store all you data in events, so you can register, repair, rebuild, and analyze your data.
monee has two levels of transactions:

- events: low level transactions that express raw money movements.
- procedures: atomic group of events that are executed together and provide more information about the transaction, e.g. transferences, buys, sales, payments, money conversion, etc.

## Future plans

Monee is planned to be a self-hosted service that can be reached from any device in a secure way.

## Contributing

We welcome issues and pull requests from anyone. While contributions are valuable and appreciated, the monee team retains full control over the project. 
Significant contributions will be acknowledged in the project’s documentation and release notes, but all official project credits and decision-making authority are reserved for the monee team.
