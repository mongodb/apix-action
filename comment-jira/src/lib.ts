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

export async function main() {
  try {
    const token = core.getInput('token');
    const issueKey = core.getInput('issue-key');
    const comment = core.getInput('comment');
    const apiBase = core.getInput('api-base');

    const requestBody = {
      body: comment.replace(/(\r\n|\n)/g, '\r\n')
    };

    const response = await fetch(`${apiBase}/rest/api/2/issue/${issueKey}/comment`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(requestBody)
    });
    const responseBody = await response.json();

    if (!response.ok) {
      throw new Error(`${response.status} ${response.statusText}\n${JSON.stringify(responseBody)}`);
    }
  } catch (error) {
    if (error instanceof Error) {
      core.setFailed(error);
    } else {
      core.setFailed(JSON.stringify(error));
    }
  }
}
