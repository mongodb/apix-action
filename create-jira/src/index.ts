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

import * as core from '@actions/core';
import request from 'request';
import { merge, mergeWith, isArray } from 'lodash';

export interface JiraIssue {
  fields: {
    project: { key: string };
    summary: string;
    description?: string;
    issuetype?: { name: string };
    assignee?: { name: string };
    labels?: string[];
    components?: { name: string }[];
    [key: string]: any;
  };
}

export function main() {
  try {
    const token = core.getInput('token');
    const projectKey = core.getInput('project-key');
    const summary = core.getInput('summary');
    const description = core.getInput('description');
    const issuetype = core.getInput('issuetype');
    const labels = core.getInput('labels');
    const components = core.getInput('components');
    const assignee = core.getInput('assignee');
    const extraData = core.getInput('extra-data');
    const apiBase = core.getInput('api-base');
    const labelList = labels.split(",").filter(value => value != "");
    const componentList = components.split(",").filter(value => value != "");

    let body: JiraIssue = { fields: { project: { key: projectKey }, summary: summary } };
    if (description != "") {
      body.fields.description = description;
    }
    if (issuetype != "") {
      body.fields.issuetype = { name: issuetype };
    }
    if (assignee != "") {
      body.fields.assignee = { name: assignee };
    }
    if (labelList.length != 0) {
      body.fields.labels = labelList;
    }
    if (componentList.length != 0) {
      body.fields.components = componentList.map(value => {
        return { name: value };
      });
    }
    if (extraData != "") {
      const extraDataObject = JSON.parse(extraData);
      mergeWith(body, extraDataObject, (dst, src) => {
        if (isArray(dst)) {
          return dst.concat(src);
        }
      });
    }

    request({
      url: `${apiBase}/rest/api/2/issue`,
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: "Bearer " + token,
      },
      body: body,
      json: true
    }, (err: any, response: request.Response, responseBody: any) => {
      if (err != null) {
        return core.setFailed(err);
      }
      if (response.statusCode >= 400) {
        return core.setFailed(response.statusCode + " " + response.statusMessage);
      }
      core.setOutput("issue-key", responseBody.key);
    });
  } catch (error) {
    if (error instanceof Error) {
      core.setFailed(error.message);
    } else {
      core.setFailed(String(error));
    }
  }
}

main();