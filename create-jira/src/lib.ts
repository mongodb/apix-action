// Copyright 2025 MongoDB Inc
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
import { mergeWith, isArray } from 'lodash';

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

interface JiraTransition {
  id: string;
  to?: { name?: string };
}

function buildRequestBody(projectKey: string, summary: string, description: string, issuetype: string, labels: string[], components: string[], assignee: string, extraData?: { [key: string]: any }): JiraIssue {
  const body: JiraIssue = { fields: { project: { key: projectKey }, summary: summary } };
  if (description != "") {
    body.fields.description = description;
  }
  if (issuetype != "") {
    body.fields.issuetype = { name: issuetype };
  }
  if (assignee != "") {
    body.fields.assignee = { name: assignee };
  }
  if (labels.length != 0) {
    body.fields.labels = labels;
  }
  if (components.length != 0) {
    body.fields.components = components.map(value => {
      return { name: value };
    });
  }
  mergeWith(body, extraData, (dst, src) => {
    if (isArray(dst)) {
      return dst.concat(src);
    }
  });
  return body;
}

async function jiraRequest(token: string, url: string, method: string, body?: any): Promise<any> {
  return await new Promise((resolve, reject) => {
    request({
      url: url,
      method: method,
      headers: {
        "Content-Type": "application/json",
        Authorization: "Bearer " + token,
      },
      body: body,
      json: true
    }, (err: any, response: request.Response, responseBody: any) => {
      if (err != null) {
        reject(err);
        return;
      }
      if (response.statusCode >= 400) {
        reject(new Error(response.statusCode + " " + response.statusMessage + "\n" + JSON.stringify(responseBody)));
        return;
      }
      resolve(responseBody);
    });
  });
}

async function createJiraIssue(token: string, apiBase: string, projectKey: string, summary: string, description: string, issuetype: string, labels: string[], components: string[], assignee: string, extraData?: { [key: string]: any }): Promise<any> {
  const body = buildRequestBody(projectKey, summary, description, issuetype, labels, components, assignee, extraData);

  return await jiraRequest(token, `${apiBase}/rest/api/2/issue`, "POST", body);
}

async function getJiraIssueStatus(token: string, apiBase: string, issueKey: string): Promise<string | undefined> {
  const responseBody = await jiraRequest(token, `${apiBase}/rest/api/2/issue/${issueKey}?fields=status`, "GET");
  return responseBody.fields?.status?.name;
}

async function transitionJiraIssue(token: string, apiBase: string, issueKey: string, status: string): Promise<void> {
  const currentStatus = await getJiraIssueStatus(token, apiBase, issueKey);
  if (currentStatus == status) {
    return;
  }

  const responseBody = await jiraRequest(token, `${apiBase}/rest/api/2/issue/${issueKey}/transitions`, "GET");
  const transitions = responseBody.transitions ?? [];
  const transition = transitions.find((value: JiraTransition) => value.to?.name == status);
  if (transition == undefined) {
    const availableStatuses = transitions
      .map((value: JiraTransition) => value.to?.name)
      .filter((value: string | undefined): value is string => value != undefined)
      .join(", ");
    throw new Error(`No transition found for ${issueKey} from ${currentStatus ?? "current status"} to ${status}. Available statuses: ${availableStatuses || "none"}`);
  }

  await jiraRequest(token, `${apiBase}/rest/api/2/issue/${issueKey}/transitions`, "POST", { transition: { id: transition.id } });
}

export async function main() {
  try {
    const token = core.getInput('token');
    const projectKey = core.getInput('project-key');
    const summary = core.getInput('summary');
    const description = core.getInput('description');
    const issuetype = core.getInput('issuetype');
    const labels = core.getInput('labels');
    const components = core.getInput('components');
    const assignee = core.getInput('assignee');
    const status = core.getInput('status');
    const extraData = core.getInput('extra-data');
    const apiBase = core.getInput('api-base');
    const labelList = labels.split(",").filter(value => value != "");
    const componentList = components.split(",").filter(value => value != "");

    let extraDataObject: { [key: string]: any } | undefined = undefined;
    if (extraData != "") {
      try {
        extraDataObject = JSON.parse(extraData);
      } catch (error) {
        throw new Error("Error parsing extra-data: " + error);
      }
    }

    const responseBody = await createJiraIssue(token, apiBase, projectKey, summary, description, issuetype, labelList, componentList, assignee, extraDataObject);

    if (!responseBody.key) {
      throw new Error("No issue key found in response: " + JSON.stringify(responseBody));
    }

    core.setOutput("issue-key", responseBody.key);

    if (status != "") {
      await transitionJiraIssue(token, apiBase, responseBody.key, status);
    }
  } catch (error) {
    if (error instanceof Error) {
      core.setFailed(error);
    } else {
      core.setFailed(JSON.stringify(error));
    }
  }
}
