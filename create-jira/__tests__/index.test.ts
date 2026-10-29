import * as core from '@actions/core';
import { main } from '../src/lib';

jest.mock('@actions/core');

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

    const fetchMock = jest.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify({ key: 'TEST-123' }), { status: 201, statusText: 'Created' })
    );

    await main();

    expect(fetchMock).toHaveBeenCalledWith(
      'https://jira.example.com/rest/api/2/issue',
      expect.objectContaining({
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: 'Bearer fake-token',
        }
      })
    );
    expect(JSON.parse(fetchMock.mock.calls[0][1]?.body as string)).toEqual({
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
    expect(setFailedMock).not.toHaveBeenCalled();
    expect(setOutputMock).toHaveBeenCalledWith('issue-key', 'TEST-123');
  });

  it('should fail if fetch rejects', async () => {
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
    jest.spyOn(globalThis, 'fetch').mockRejectedValue(err);

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

    const responseBody = {
      detail: "Bad Request"
    };
    jest.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify(responseBody), { status: 400, statusText: 'Bad Request' })
    );

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
