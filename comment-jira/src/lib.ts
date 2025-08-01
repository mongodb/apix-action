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

export async function main() {
  try {
    const token = core.getInput('token');
    const issueKey = core.getInput('issue-key');
    const comment = core.getInput('comment');
    const apiBase = core.getInput('api-base');

    const requestBody = {
      body: comment.replace(/(\r\n|\n)/g, '\r\n')
    };

    await new Promise((resolve, reject) => {
      request({
        url: `${apiBase}/rest/api/2/issue/${issueKey}/comment`,
        method: "POST",
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json'
        },
        body: requestBody,
        json: true
      }, (err: any, response: request.Response, responseBody: any) => {
        if (err) {
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
  } catch (error) {
    if (error instanceof Error) {
      core.setFailed(error);
    } else {
      core.setFailed(JSON.stringify(error));
    }
  }
}
