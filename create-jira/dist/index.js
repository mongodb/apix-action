"use strict";
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
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const core = __importStar(require("@actions/core"));
const request_1 = __importDefault(require("request"));
const lodash = __importStar(require("lodash"));
function main() {
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
        let body = { fields: { project: { key: projectKey }, summary: summary } };
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
            lodash.mergeWith(body, extraDataObject, (dst, src) => {
                if (lodash.isArray(dst)) {
                    return dst.concat(src);
                }
            });
        }
        (0, request_1.default)({
            url: `${apiBase}/rest/api/2/issue`,
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
            if (response.statusCode >= 400) {
                return core.setFailed(response.statusCode + " " + response.statusMessage);
            }
            core.setOutput("issue-key", responseBody.key);
        });
    }
    catch (error) {
        if (error instanceof Error) {
            core.setFailed(error.message);
        }
        else {
            core.setFailed(String(error));
        }
    }
}
main();
