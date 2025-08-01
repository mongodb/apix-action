import * as core from '@actions/core';
import request from 'request';
import { main } from '../src/lib';

jest.mock('@actions/core');
jest.mock('request');

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

    const responseMock = {
      statusCode: 200,
      statusMessage: 'OK'
    };

    const responseBody = { id: '10000' };

    let requestBody: any = null;
    let requestUrl: string = '';
    let requestHeaders: any = null;

    (request as unknown as jest.Mock).mockImplementation((options, callback) => {
      requestUrl = options.url;
      requestHeaders = options.headers;
      requestBody = options.body;
      callback(null, responseMock, responseBody);
    });

    await main();

    expect(requestUrl).toBe('https://jira.example.com/rest/api/2/issue/TEST-123/comment');
    expect(requestHeaders).toEqual({
      'Authorization': 'Bearer fake-token',
      'Content-Type': 'application/json'
    });
    expect(requestBody).toEqual({
      body: 'This is a test comment\r\nwith multiple lines\r\n'
    });
    expect(setFailedMock).not.toHaveBeenCalled();
  });

  it('should fail if request returns an error', async () => {
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
});
