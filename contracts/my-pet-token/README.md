# My Pet Token

## `execute` Function Documentation

The `execute` function serves as the entry point for interacting with the contract. It processes incoming messages and executes corresponding actions based on the message type.

### Message Types:

1. **Transfer**:
   - **Purpose**: Transfers a specified amount of tokens to the given recipient.
   - **Parameters**:
     - `recipient`: Address of the recipient.
     - `amount`: Amount of tokens to transfer.
   - **Execution**: Calls the `execute_transfer` function.

2. **Mint**:
   - **Purpose**: Mints a specified amount of tokens and assigns them to the recipient.
   - **Parameters**:
     - `recipient`: Address of the recipient.
     - `amount`: Amount of tokens to mint.
   - **Execution**: Calls the `execute_mint` function.

3. **Increase Allowance**:
   - **Purpose**: Increases the spender's allowance to spend tokens on behalf of the owner.
   - **Parameters**:
     - `spender`: Address of the spender.
     - `amount`: Amount by which to increase the allowance.
     - `expires`: Expiry time for the allowance.
   - **Execution**: Calls the `execute_increase_allowance` function.

4. **Decrease Allowance**:
   - **Purpose**: Decreases the spender's allowance to spend tokens on behalf of the owner.
   - **Parameters**:
     - `spender`: Address of the spender.
     - `amount`: Amount by which to decrease the allowance.
     - `expires`: Expiry time for the allowance.
   - **Execution**: Calls the `execute_decrease_allowance` function.

5. **Transfer From**:
   - **Purpose**: Transfers tokens from one account to another on behalf of the owner.
   - **Parameters**:
     - `owner`: Address of the token owner.
     - `recipient`: Address of the recipient.
     - `amount`: Amount of tokens to transfer.
   - **Execution**: Calls the `execute_transfer_from` function.

6. **Update Minter**:
   - **Purpose**: Updates the address authorized to mint new tokens.
   - **Parameters**:
     - `new_minter`: New address authorized to mint tokens.
   - **Execution**: Calls the `execute_update_minter` function.

### Error Handling:

- If any error occurs during message processing, a `ContractError` is returned.

### Return Value:

- Returns a `Result` containing either a `Response` indicating the success of the operation or a `ContractError` if an error occurs.

---

This function serves as the primary endpoint for interacting with the contract, allowing users to perform various token-related operations such as transfers, minting, allowance management, and minter authorization updates.