import * as core from '@actions/core';
import * as request from 'request';
import { main, JiraIssue } from '../src/index';

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

    main();

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
    expect(setOutputMock).toHaveBeenCalledWith('issue-key', 'TEST-123');
    expect(setFailedMock).not.toHaveBeenCalled();
  });

  it('should fail if request returns an error', () => {
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

    main();

    expect(setFailedMock).toHaveBeenCalledWith(err);
  });

  it('should fail if response status code is >= 400', () => {
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

    main();

    expect(setFailedMock).toHaveBeenCalledWith("400 Bad Request\n{\"detail\":\"Bad Request\"}");
  });
  it('should fail if request returns an error', () => {
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
      callback(err);
    });

    main();

    expect(setFailedMock).toHaveBeenCalledWith(err);
  });

  it('should fail if json is invalid', () => {
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

    main();

    expect(setFailedMock).toHaveBeenCalledWith("Error parsing extra-data: SyntaxError: Expected property name or '}' in JSON at position 1 (line 1 column 2)");
  });
});
