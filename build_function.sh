#!/bin/bash

cd functions/transiterProxy/
cargo lambda build --release --arm64
cd target/lambda/resolver
zip bootstrap.zip bootstrap
