import { Duration, Stack, StackProps } from "aws-cdk-lib";
import { Construct } from "constructs";

import * as appsync from "@aws-cdk/aws-appsync-alpha";
import * as lambda from "aws-cdk-lib/aws-lambda";
import * as logs from "aws-cdk-lib/aws-logs";
import * as ec2 from "aws-cdk-lib/aws-ec2";

export class TransiterGraphqlProxyStack extends Stack {
  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, id, props);

    const vpc = ec2.Vpc.fromLookup(this, "Vpc", {
      region: process.env.CDK_DEFAULT_REGION,
      vpcId: process.env.TRANSITER_VPC!,
    });

    const graphQlApi = new appsync.GraphqlApi(
      this,
      "TransiterGraphQLProxyAPI",
      {
        name: "TransiterGraphQLProxyAPI",
        schema: appsync.Schema.fromAsset("graphql/schema.graphql"),
        authorizationConfig: {
          defaultAuthorization: {
            authorizationType: appsync.AuthorizationType.IAM,
          },
          additionalAuthorizationModes: [
            {
              authorizationType: appsync.AuthorizationType.API_KEY,
            },
          ],
        },
      }
    );

    const transiterProxyLambda = new lambda.Function(this, 'TransiterProxyLambda', {
        vpc,
        code: lambda.Code.fromAsset(
            'functions/transiterProxy/target/lambda/resolver/bootstrap.zip'
        ),
        runtime: lambda.Runtime.PROVIDED_AL2,
        architecture: lambda.Architecture.ARM_64,
        handler: 'not.required',
        environment: {
            RUST_BACKTRACE: '1',
            TRANSITER_HOST: process.env.TRANSITER_HOST!,
        },
        logRetention: logs.RetentionDays.ONE_WEEK,
        timeout: Duration.seconds(10),
      })

    const transiterProxyLambdaDataSource = graphQlApi.addLambdaDataSource(
      "TransiterProxyLambdaDataSource",
      transiterProxyLambda
    );

    transiterProxyLambdaDataSource.createResolver({
      typeName: "Query",
      fieldName: "nearbyTrainTimes",
    });

    transiterProxyLambdaDataSource.createResolver({
      typeName: "Query",
      fieldName: "routeStatuses",
    });
  }
}
