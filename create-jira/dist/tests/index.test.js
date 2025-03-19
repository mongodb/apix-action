"use strict";
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
Object.defineProperty(exports, "__esModule", { value: true });
const core = __importStar(require("@actions/core"));
const request = __importStar(require("request"));
const index_1 = require("../src/index");
jest.mock('@actions/core');
jest.mock('request');
describe('Create Jira Issue Action', () => {
    beforeEach(() => {
        jest.clearAllMocks();
    });
    it('should create a Jira issue successfully', () => {
        const setOutputMock = jest.spyOn(core, 'setOutput');
        const setFailedMock = jest.spyOn(core, 'setFailed');
        const getInputMock = jest.spyOn(core, 'getInput');
        getInputMock.mockImplementation((name) => {
            switch (name) {
                case 'token':
                    return 'fake-token';
                case 'project-key':
                    return 'TEST';
                case 'summary':
                    return 'Test issue';
                case 'description':
                    return 'Test description';
                case 'issuetype':
                    return 'Task';
                case 'labels':
                    return 'bug,urgent';
                case 'components':
                    return 'backend,frontend';
                case 'assignee':
                    return 'test-user';
                case 'extra-data':
                    return '{"customfield_10011": "custom-value"}';
                case 'api-base':
                    return 'https://jira.example.com';
                default:
                    return '';
            }
        });
        const responseMock = {
            statusCode: 201,
            statusMessage: 'Created',
            body: { key: 'TEST-123' }
        };
        let requestBody = null;
        request.mockImplementation((options, callback) => {
            const { body } = options;
            requestBody = body;
            callback(null, responseMock, responseMock.body);
        });
        (0, index_1.main)();
        expect(requestBody).toEqual({
            fields: {
                project: { key: 'TEST' },
                summary: 'Test issue',
                description: 'Test description',
                issuetype: { name: 'Task' },
                assignee: { name: 'test-user' },
                components: [{ name: 'backend' }, { name: 'frontend' }],
                labels: ["bug", "urgent"]
            },
            customfield_10011: 'custom-value'
        });
        expect(setOutputMock).toHaveBeenCalledWith('issue-key', 'TEST-123');
        expect(setFailedMock).not.toHaveBeenCalled();
    });
    it('should fail if request returns an error', () => {
        const setFailedMock = jest.spyOn(core, 'setFailed');
        const getInputMock = jest.spyOn(core, 'getInput');
        getInputMock.mockImplementation((name) => {
            switch (name) {
                case 'token':
                    return 'fake-token';
                case 'project-key':
                    return 'TEST';
                case 'summary':
                    return 'Test issue';
                case 'description':
                    return 'Test description';
                case 'issuetype':
                    return 'Task';
                case 'labels':
                    return 'bug,urgent';
                case 'components':
                    return 'backend,frontend';
                case 'assignee':
                    return 'test-user';
                case 'extra-data':
                    return '{"customfield_10011": "custom-value"}';
                case 'api-base':
                    return 'https://jira.example.com';
                default:
                    return '';
            }
        });
        const err = new Error('Request failed');
        request.mockImplementation((options, callback) => {
            callback(err, null, null);
        });
        (0, index_1.main)();
        expect(setFailedMock).toHaveBeenCalledWith(err);
    });
    it('should fail if response status code is >= 400', () => {
        const setFailedMock = jest.spyOn(core, 'setFailed');
        const getInputMock = jest.spyOn(core, 'getInput');
        getInputMock.mockImplementation((name) => {
            switch (name) {
                case 'token':
                    return 'fake-token';
                case 'project-key':
                    return 'TEST';
                case 'summary':
                    return 'Test issue';
                case 'description':
                    return 'Test description';
                case 'issuetype':
                    return 'Task';
                case 'labels':
                    return 'bug,urgent';
                case 'components':
                    return 'backend,frontend';
                case 'assignee':
                    return 'test-user';
                case 'extra-data':
                    return '{"customfield_10011": "custom-value"}';
                case 'api-base':
                    return 'https://jira.example.com';
                default:
                    return '';
            }
        });
        const responseMock = {
            statusCode: 400,
            statusMessage: 'Bad Request',
        };
        const responseBody = {};
        request.mockImplementation((options, callback) => {
            callback(null, responseMock, responseBody);
        });
        (0, index_1.main)();
        expect(setFailedMock).toHaveBeenCalledWith("400 Bad Request");
    });
});
