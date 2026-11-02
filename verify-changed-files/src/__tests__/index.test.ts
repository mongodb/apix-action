import { afterEach, beforeEach, describe, expect, test, vi, type MockInstance } from 'vitest';
import * as core from '@actions/core';
import * as exec from '@actions/exec';
import { run } from '../index'; // Update the index.ts to export the run function

vi.mock('@actions/core', () => ({
  getInput: vi.fn(),
  setOutput: vi.fn(),
  setFailed: vi.fn(),
  info: vi.fn(),
  debug: vi.fn()
}));
vi.mock('@actions/exec', () => ({ exec: vi.fn() }));

describe('verify-changed-files action', () => {
  let mockGetInput: MockInstance<typeof core.getInput>;
  let mockSetOutput: MockInstance<typeof core.setOutput>;
  let mockSetFailed: MockInstance<typeof core.setFailed>;
  let mockInfo: MockInstance<typeof core.info>;
  let mockDebug: MockInstance<typeof core.debug>;
  let mockExec: MockInstance<typeof exec.exec>;

  beforeEach(() => {
    // Setup core mocks
    mockGetInput = vi.spyOn(core, 'getInput').mockImplementation((name) => {
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
    mockSetOutput = vi.spyOn(core, 'setOutput').mockImplementation(() => undefined);
    mockSetFailed = vi.spyOn(core, 'setFailed').mockImplementation(() => undefined);
    mockInfo = vi.spyOn(core, 'info').mockImplementation(() => undefined);
    mockDebug = vi.spyOn(core, 'debug').mockImplementation(() => undefined);
    
    // Setup exec mock
    mockExec = vi.spyOn(exec, 'exec').mockImplementation(async (cmd, args, options) => {
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
    vi.resetAllMocks();
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
    vi.resetAllMocks();

    // Re-setup core mocks for this test
    mockGetInput = vi.spyOn(core, 'getInput').mockImplementation((name) => {
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
    mockSetOutput = vi.spyOn(core, 'setOutput').mockImplementation(() => undefined);
    mockSetFailed = vi.spyOn(core, 'setFailed').mockImplementation(() => undefined);
    mockInfo = vi.spyOn(core, 'info').mockImplementation(() => undefined);
    mockDebug = vi.spyOn(core, 'debug').mockImplementation(() => undefined);
    
    // Override exec mock to return empty for all commands
    mockExec = vi.spyOn(exec, 'exec').mockImplementation(async (cmd, args, options) => {
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
    vi.resetAllMocks();

    // Setup core mocks without files input
    mockGetInput = vi.spyOn(core, 'getInput').mockImplementation((name) => {
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
    mockSetOutput = vi.spyOn(core, 'setOutput').mockImplementation(() => undefined);
    mockInfo = vi.spyOn(core, 'info').mockImplementation(() => undefined);
    
    // Mock exec with some changed files
    mockExec = vi.spyOn(exec, 'exec').mockImplementation(async (cmd, args, options) => {
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
    vi.resetAllMocks();
    
    // Make getInput throw an error
    mockGetInput = vi.spyOn(core, 'getInput').mockImplementationOnce(() => {
      throw new Error('Test error');
    });
    mockSetFailed = vi.spyOn(core, 'setFailed').mockImplementation(() => undefined);
    
    await run();
    
    // Should call setFailed
    expect(mockSetFailed).toHaveBeenCalledWith('Test error');
  });
});
