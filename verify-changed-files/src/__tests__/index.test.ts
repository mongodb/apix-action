import { jest as jestObject } from '@jest/globals';
import * as core from '@actions/core';
import * as exec from '@actions/exec';
import { run } from '../index'; // Update the index.ts to export the run function

// Mock the @actions/core and @actions/exec modules
jestObject.mock('@actions/core');
jestObject.mock('@actions/exec');

describe('verify-changed-files action', () => {
  let mockGetInput: jest.SpyInstance;
  let mockSetOutput: jest.SpyInstance;
  let mockSetFailed: jest.SpyInstance;
  let mockInfo: jest.SpyInstance;
  let mockDebug: jest.SpyInstance;
  let mockExec: jest.SpyInstance;

  beforeEach(() => {
    // Setup core mocks
    mockGetInput = jestObject.spyOn(core, 'getInput').mockImplementation((name) => {
      switch (name) {
        case 'files':
          return 'src/**/*.ts\npackage.json';
        case 'base-ref':
          return 'HEAD^';
        case 'head-ref':
          return 'HEAD';
        case 'separator':
          return ' ';
        default:
          return '';
      }
    });
    mockSetOutput = jestObject.spyOn(core, 'setOutput').mockImplementation(() => undefined);
    mockSetFailed = jestObject.spyOn(core, 'setFailed').mockImplementation(() => undefined);
    mockInfo = jestObject.spyOn(core, 'info').mockImplementation(() => undefined);
    mockDebug = jestObject.spyOn(core, 'debug').mockImplementation(() => undefined);
    
    // Setup exec mock
    mockExec = jestObject.spyOn(exec, 'exec').mockImplementation(async (cmd, args, options) => {
      // Mock different Git responses based on command
      if (args && args.includes('diff') && !args.includes('--')) {
        // Main git diff command
        options?.listeners?.stdout?.(Buffer.from('src/index.ts\npackage.json\nREADME.md'));
      } else if (args && args.includes('--') && args.includes('src/**/*.ts')) {
        // Git diff for src/**/*.ts pattern
        options?.listeners?.stdout?.(Buffer.from('src/index.ts\nsrc/utils.ts'));
      } else if (args && args.includes('--') && args.includes('package.json')) {
        // Git diff for package.json
        options?.listeners?.stdout?.(Buffer.from('package.json'));
      }
      return 0;
    });
  });

  afterEach(() => {
    jestObject.resetAllMocks();
  });

  test('detects changed files correctly', async () => {
    await run();
    
    // Check outputs
    expect(mockSetOutput).toHaveBeenCalledWith('changed_files', expect.stringContaining('src/index.ts'));
    expect(mockSetOutput).toHaveBeenCalledWith('changed_files', expect.stringContaining('package.json'));
    expect(mockSetOutput).toHaveBeenCalledWith('files_changed', 'true');
    expect(mockSetOutput).toHaveBeenCalledWith('all_files_changed', 'true');
    
    // Check logs
    expect(mockInfo).toHaveBeenCalled();
    expect(mockSetFailed).not.toHaveBeenCalled();
  });

  test('handles case with no changed files', async () => {
    // Reset mocks to clear previous test state
    jestObject.resetAllMocks();

    // Re-setup core mocks for this test
    mockGetInput = jestObject.spyOn(core, 'getInput').mockImplementation((name) => {
      switch (name) {
        case 'files':
          return 'src/**/*.ts\npackage.json';
        case 'base-ref':
          return 'HEAD^';
        case 'head-ref':
          return 'HEAD';
        case 'separator':
          return ' ';
        default:
          return '';
      }
    });
    mockSetOutput = jestObject.spyOn(core, 'setOutput').mockImplementation(() => undefined);
    mockSetFailed = jestObject.spyOn(core, 'setFailed').mockImplementation(() => undefined);
    mockInfo = jestObject.spyOn(core, 'info').mockImplementation(() => undefined);
    mockDebug = jestObject.spyOn(core, 'debug').mockImplementation(() => undefined);
    
    // Override exec mock to return empty for all commands
    mockExec = jestObject.spyOn(exec, 'exec').mockImplementation(async (cmd, args, options) => {
      // Always return empty string for all git commands
      options?.listeners?.stdout?.(Buffer.from(''));
      return 0;
    });
    
    await run();
    
    // Check outputs for no changes
    expect(mockSetOutput).toHaveBeenCalledWith('changed_files', '');
    expect(mockSetOutput).toHaveBeenCalledWith('files_changed', 'false');
    expect(mockSetOutput).toHaveBeenCalledWith('all_files_changed', 'false');
  });

  test('uses default value when files input is not provided', async () => {
    // Reset mocks to clear previous test state
    jestObject.resetAllMocks();

    // Setup core mocks without files input
    mockGetInput = jestObject.spyOn(core, 'getInput').mockImplementation((name) => {
      switch (name) {
        case 'files':
          return '';  // Empty input to test default behavior
        case 'base-ref':
          return 'HEAD^';
        case 'head-ref':
          return 'HEAD';
        case 'separator':
          return ' ';
        default:
          return '';
      }
    });
    mockSetOutput = jestObject.spyOn(core, 'setOutput').mockImplementation(() => undefined);
    mockInfo = jestObject.spyOn(core, 'info').mockImplementation(() => undefined);
    
    // Mock exec with some changed files
    mockExec = jestObject.spyOn(exec, 'exec').mockImplementation(async (cmd, args, options) => {
      // When checking the entire repo, return some files
      if (args && args.includes('diff') && !args.includes('--')) {
        options?.listeners?.stdout?.(Buffer.from('README.md\nLICENSE'));
      } else if (args && args.includes('--') && args.includes('.')) {
        options?.listeners?.stdout?.(Buffer.from('README.md\nLICENSE'));
      }
      return 0;
    });
    
    await run();
    
    // Check outputs for default behavior
    expect(mockSetOutput).toHaveBeenCalledWith('changed_files', expect.stringContaining('README.md'));
    expect(mockSetOutput).toHaveBeenCalledWith('files_changed', 'true');
  });

  test('handles errors gracefully', async () => {
    // Reset mocks to clear previous test state
    jestObject.resetAllMocks();
    
    // Make getInput throw an error
    mockGetInput = jestObject.spyOn(core, 'getInput').mockImplementationOnce(() => {
      throw new Error('Test error');
    });
    mockSetFailed = jestObject.spyOn(core, 'setFailed').mockImplementation(() => undefined);
    
    await run();
    
    // Should call setFailed
    expect(mockSetFailed).toHaveBeenCalledWith('Test error');
  });
});
