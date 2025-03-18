import * as core from '@actions/core';
import * as exec from '@actions/exec';
import { run } from '../index'; // Update the index.ts to export the run function

// Mock the @actions/core and @actions/exec modules
jest.mock('@actions/core');
jest.mock('@actions/exec');

describe('verify-changed-files action', () => {
  let mockGetInput: jest.SpyInstance;
  let mockSetOutput: jest.SpyInstance;
  let mockSetFailed: jest.SpyInstance;
  let mockInfo: jest.SpyInstance;
  let mockDebug: jest.SpyInstance;
  let mockExec: jest.SpyInstance;

  beforeEach(() => {
    // Setup core mocks
    mockGetInput = jest.spyOn(core, 'getInput').mockImplementation((name) => {
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
    mockSetOutput = jest.spyOn(core, 'setOutput').mockImplementation();
    mockSetFailed = jest.spyOn(core, 'setFailed').mockImplementation();
    mockInfo = jest.spyOn(core, 'info').mockImplementation();
    mockDebug = jest.spyOn(core, 'debug').mockImplementation();
    
    // Setup exec mock
    mockExec = jest.spyOn(exec, 'exec').mockImplementation(async (cmd, args, options) => {
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
    jest.resetAllMocks();
  });

  test('detects changed files correctly', async () => {
    await run();
    
    // Check outputs
    expect(mockSetOutput).toHaveBeenCalledWith('changed_files', expect.stringContaining('src/index.ts'));
    expect(mockSetOutput).toHaveBeenCalledWith('changed_files', expect.stringContaining('package.json'));
    expect(mockSetOutput).toHaveBeenCalledWith('any_changed', 'true');
    expect(mockSetOutput).toHaveBeenCalledWith('all_changed', 'true');
    
    // Check logs
    expect(mockInfo).toHaveBeenCalled();
    expect(mockSetFailed).not.toHaveBeenCalled();
  });

  test('handles case with no changed files', async () => {
    // Reset mocks to clear previous test state
    jest.resetAllMocks();

    // Re-setup core mocks for this test
    mockGetInput = jest.spyOn(core, 'getInput').mockImplementation((name) => {
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
    mockSetOutput = jest.spyOn(core, 'setOutput').mockImplementation();
    mockSetFailed = jest.spyOn(core, 'setFailed').mockImplementation();
    mockInfo = jest.spyOn(core, 'info').mockImplementation();
    mockDebug = jest.spyOn(core, 'debug').mockImplementation();
    
    // Override exec mock to return empty for all commands
    mockExec = jest.spyOn(exec, 'exec').mockImplementation(async (cmd, args, options) => {
      // Always return empty string for all git commands
      options?.listeners?.stdout?.(Buffer.from(''));
      return 0;
    });
    
    await run();
    
    // Check outputs for no changes
    expect(mockSetOutput).toHaveBeenCalledWith('changed_files', '');
    expect(mockSetOutput).toHaveBeenCalledWith('any_changed', 'false');
    expect(mockSetOutput).toHaveBeenCalledWith('all_changed', 'false');
  });

  test('uses default value when files input is not provided', async () => {
    // Reset mocks to clear previous test state
    jest.resetAllMocks();

    // Setup core mocks without files input
    mockGetInput = jest.spyOn(core, 'getInput').mockImplementation((name) => {
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
    mockSetOutput = jest.spyOn(core, 'setOutput').mockImplementation();
    mockInfo = jest.spyOn(core, 'info').mockImplementation();
    
    // Mock exec with some changed files
    mockExec = jest.spyOn(exec, 'exec').mockImplementation(async (cmd, args, options) => {
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
    expect(mockSetOutput).toHaveBeenCalledWith('any_changed', 'true');
  });

  test('handles errors gracefully', async () => {
    // Reset mocks to clear previous test state
    jest.resetAllMocks();
    
    // Make getInput throw an error
    mockGetInput = jest.spyOn(core, 'getInput').mockImplementationOnce(() => {
      throw new Error('Test error');
    });
    mockSetFailed = jest.spyOn(core, 'setFailed').mockImplementation();
    
    await run();
    
    // Should call setFailed
    expect(mockSetFailed).toHaveBeenCalledWith('Test error');
  });
});
