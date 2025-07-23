import * as core from '@actions/core';
import * as request from 'request';
import { main, JiraIssue } from '../src/index';

jest.mock('@actions/core');
jest.mock('request');

describe('Create Jira Issue Action', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('should create a Jira issue successfully', async () => {
    const setOutputMock = jest.spyOn(core, 'setOutput');
    const setFailedMock = jest.spyOn(core, 'setFailed');
    const getInputMock = jest.spyOn(core, 'getInput');

    getInputMock.mockImplementation((name: string) => {
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

    let requestBody: any = null;

    (request as unknown as jest.Mock).mockImplementation((options, callback) => {
      const { body } = options;
      requestBody = body;
      callback(null, responseMock, responseMock.body);
    });

    await main();

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
    } as JiraIssue)
    expect(setFailedMock).not.toHaveBeenCalled();
    expect(setOutputMock).toHaveBeenCalledWith('issue-key', 'TEST-123');
  });

  it('should fail if request returns an error', async () => {
    const setFailedMock = jest.spyOn(core, 'setFailed');
    const getInputMock = jest.spyOn(core, 'getInput');

    getInputMock.mockImplementation((name: string) => {
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

    (request as unknown as jest.Mock).mockImplementation((options, callback) => {
      callback(err, null, null);
    });

    await main();

    expect(setFailedMock).toHaveBeenCalledWith(err);
  });

  it('should fail if response status code is >= 400', async () => {
    const setFailedMock = jest.spyOn(core, 'setFailed');
    const getInputMock = jest.spyOn(core, 'getInput');

    getInputMock.mockImplementation((name: string) => {
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

    const responseBody = {
      detail: "Bad Request"
    };

    (request as unknown as jest.Mock).mockImplementation((options, callback) => {
      callback(null, responseMock, responseBody);
    });

    await main();

    const err = new Error("400 Bad Request\n{\"detail\":\"Bad Request\"}");

    expect(setFailedMock).toHaveBeenCalledWith(err);
  });
  it('should fail if json is invalid', async () => {
    const setFailedMock = jest.spyOn(core, 'setFailed');
    const getInputMock = jest.spyOn(core, 'getInput');

    getInputMock.mockImplementation((name: string) => {
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
          return '{invalid-json';
        case 'api-base':
          return 'https://jira.example.com';
        default:
          return '';
      }
    });

    await main();

    const err = new Error("Error parsing extra-data: SyntaxError: Expected property name or '}' in JSON at position 1 (line 1 column 2)");

    expect(setFailedMock).toHaveBeenCalledWith(err);
  });
});
