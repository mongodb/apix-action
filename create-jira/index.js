// Copyright 2024 MongoDB Inc
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

const core = require('@actions/core');
const request = require('request');
const lodash = require('lodash');

function main() {
  try {
    const token = core.getInput('token');
    const projectKey = core.getInput('project-key');
    const summary = core.getInput('summary');
    const description = core.getInput('description');
    const issuetype = core.getInput('issuetype');
    const labels = core.getInput('labels');
    const components = core.getInput('components');
    const extraData = core.getInput('extra-data');
    const labelList = labels.split(",").filter(value => value != "")
    const componentList = components.split(",").filter(value => value != "")

    body = {"fields":{"project":{"key":projectKey},"summary":summary}};
    if (description != "") {
      body["fields"]["description"] = description;
    }
    if (issuetype != "") {
      body["fields"]["issuetype"] = {"name": issuetype};
    }
    if (labelList.length != 0) {
      body["fields"]["labels"] = labelList;
    }
    if (componentList.length != 0) {
      body["fields"]["components"] = componentList.map(value => {
        return {"name": value};
      });
    }
    if (extraData != "") {
      const extraDataObject = JSON.parse(extraData);
      lodash.mergeWith(body, extraDataObject, (dst, src) => {
        if (lodash.isArray(dst)) {
          return dst.concat(src);
        }
      })
    }
    console.info("sending: " + JSON.stringify(body));

    request({
      url: "https://jira.mongodb.org/rest/api/2/issue",
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: "Bearer " + token,
      },
      body: body,
      json: true
    }, (err, response, responseBody) => {
      if (err != null) {
        return core.setFailed(err);
      }
      console.info("received: " + JSON.stringify(responseBody));
      if (response.statusCode >= 400) {
        return core.setFailed(response.statusCode + " " + response.statusMessage);
      }
      core.setOutput("issue-key", responseBody.key);
    });
  } catch (error) {
    core.setFailed(error.message);
  }
}

main();
