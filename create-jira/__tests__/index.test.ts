import * as core from '@actions/core';
import * as request from 'request';
import { main, JiraIssue } from '../src/lib';

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
    expect(request as unknown as jest.Mock).toHaveBeenCalledTimes(1);
    expect(setFailedMock).not.toHaveBeenCalled();
    expect(setOutputMock).toHaveBeenCalledWith('issue-key', 'TEST-123');
  });

  it('should transition a Jira issue when status is provided', async () => {
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
        case 'status':
          return 'In Progress';
        case 'extra-data':
          return '{"customfield_10011": "custom-value"}';
        case 'api-base':
          return 'https://jira.example.com';
        default:
          return '';
      }
    });

    const requests: any[] = [];

    (request as unknown as jest.Mock).mockImplementation((options, callback) => {
      requests.push(options);
      switch (requests.length) {
        case 1:
          callback(null, { statusCode: 201, statusMessage: 'Created' }, { key: 'TEST-123' });
          return;
        case 2:
          callback(null, { statusCode: 200, statusMessage: 'OK' }, { fields: { status: { name: 'Needs Triage' } } });
          return;
        case 3:
          callback(null, { statusCode: 200, statusMessage: 'OK' }, { transitions: [{ id: '11', to: { name: 'Done' } }, { id: '21', to: { name: 'In Progress' } }] });
          return;
        case 4:
          callback(null, { statusCode: 204, statusMessage: 'No Content' }, undefined);
          return;
        default:
          throw new Error('Unexpected request');
      }
    });

    await main();

    expect(requests).toHaveLength(4);
    expect(requests[0]).toMatchObject({
      url: 'https://jira.example.com/rest/api/2/issue',
      method: 'POST',
    });
    expect(requests[1]).toMatchObject({
      url: 'https://jira.example.com/rest/api/2/issue/TEST-123?fields=status',
      method: 'GET',
    });
    expect(requests[2]).toMatchObject({
      url: 'https://jira.example.com/rest/api/2/issue/TEST-123/transitions',
      method: 'GET',
    });
    expect(requests[3]).toMatchObject({
      url: 'https://jira.example.com/rest/api/2/issue/TEST-123/transitions',
      method: 'POST',
      body: { transition: { id: '21' } },
    });
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

    expect(setFailedMock).toHaveBeenCalledTimes(1);
    const error = setFailedMock.mock.calls[0][0] as Error;
    expect(error).toBeInstanceOf(Error);
    expect(error.message).toContain(
      "Error parsing extra-data: SyntaxError: Expected property name or '}' in JSON at position 1"
    );
  });
});
