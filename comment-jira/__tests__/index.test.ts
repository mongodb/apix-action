import * as core from '@actions/core';
import { main } from '../src/lib';

jest.mock('@actions/core');

describe('Comment Jira Issue Action', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('should comment on a Jira issue successfully', async () => {
    const setFailedMock = jest.spyOn(core, 'setFailed');
    const getInputMock = jest.spyOn(core, 'getInput');

    getInputMock.mockImplementation((name: string) => {
      switch (name) {
        case 'token':
          return 'fake-token';
        case 'issue-key':
          return 'TEST-123';
        case 'comment':
          return 'This is a test comment\nwith multiple lines\n';
        case 'api-base':
          return 'https://jira.example.com';
        default:
          return '';
      }
    });

    const responseBody = { id: '10000' };
    const fetchMock = jest.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify(responseBody), { status: 200, statusText: 'OK' })
    );

    await main();

    expect(fetchMock).toHaveBeenCalledWith(
      'https://jira.example.com/rest/api/2/issue/TEST-123/comment',
      {
        method: 'POST',
        headers: {
          'Authorization': 'Bearer fake-token',
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          body: 'This is a test comment\r\nwith multiple lines\r\n'
        })
      }
    );
    expect(setFailedMock).not.toHaveBeenCalled();
  });

  it('should fail if fetch rejects', async () => {
    const setFailedMock = jest.spyOn(core, 'setFailed');
    const getInputMock = jest.spyOn(core, 'getInput');

    getInputMock.mockImplementation((name: string) => {
      switch (name) {
      case 'token':
        return 'fake-token';
      case 'issue-key':
        return 'TEST-123';
      case 'comment':
        return 'This is a test comment\nwith multiple lines\n';
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
        case 'issue-key':
          return 'TEST-123';
        case 'comment':
          return 'This is a test comment\nwith multiple lines\n';
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
});
