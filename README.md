# PetStaking Contract

This project contains two contracts: `mypet` and `PetStaking`. The `mypet` contract is a CW20 token, while the `PetStaking` contract is a CW4 staking contract designed to support staking functionality for the `mypet` token.

## Features

- **CW20 Token (mypet):** This contract represents the `mypet` token, which is a standard Cosmwasm token implementation compliant with CW20 specifications.

- **PetStaking Contract:** The `PetStaking` contract allows users to stake their `mypet` tokens and earn rewards. It has the following features:

  - Permission to mint `mypet` tokens.
  - Accepts `mypet` tokens for staking operations.
  - Mints 1 million `mypet` tokens per day.
  - Provides 1 million `mypet` tokens to stakers daily.
  - No token minting occurs when there's no staking activity.

## Installation and Deployment

To deploy the contracts and interact with them, follow these steps:

1. Clone this repository:

   ```bash
   git clone <repository_url>
   ```

2. Navigate to the project directory:

   ```bash
   cd <project_directory>
   ```

3. Install dependencies:

   ```bash
   cargo build
   ```

## Testing

To test the contracts, you can use the provided unit tests. Run the following command:

```bash
cargo test
```

Ensure that the tests provide good coverage for the contracts' functionality.

## Interacting with the Contracts

To interact with the deployed contracts, you can use various tools and libraries compatible with Cosmwasm contracts. Some common methods include using `cosmwasm-cli`, integrating with blockchain wallets, or building custom applications.

Ensure that you have the necessary permissions and access to interact with the contracts based on your deployment setup.

## Unit Test Coverage

The project includes comprehensive unit tests to ensure the correctness and reliability of the contracts. These tests cover various scenarios and edge cases to validate the contracts' functionality.

---

Feel free to reach out if you have any questions or need further assistance with deploying or interacting with the contracts. Happy staking! 🐾