# A GraphQL proxy for communicating with the Transiter service

This project creates an AWS AppSync endpoint for communicating with the [Transiter](https://github.com/jamespfennell/transiter) service.

It runs in AWS and is deployed using CDK.

## Setup

This project does not setup the Transiter service itself. You will need to do this first using another AWS service, such as ECS or EKS. After this is done, make note of the VPC that the Transiter service is deployed in.

## Building and deploying

### Environment

The following environment variables are used:

- `TRANSITER_VPC`: the VPC ID where Transiter is running. This service will also be deployed there.
- `TRANSITER_HOST`: this is the endpoint within the VPC that the Transiter service can be reached at. For exmaple, it may be an ALB endpoint connected to an ECS cluster running the container(s).

### Requirements

This project uses [cargo-lambda](https://github.com/cargo-lambda/cargo-lambda) to build the Lambda binary. Ensure it is installed and on your `PATH`.

### Deploying

For convenience, the script `build_function` will build the Lambda function binary and also compress the binary asset. After running this script, you can deploy via CDK:

```
./build_function.sh && cdk deploy
```
