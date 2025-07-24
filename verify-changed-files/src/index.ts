import * as core from '@actions/core';
import { exec } from '@actions/exec';

// Export the run function for testing
export async function run(): Promise<void> {
  try {
    // Get inputs
    const files = core.getInput('files') || '.'; // Default to entire repo if not specified
    let baseRef = core.getInput('base-ref');
    const headRef = core.getInput('head-ref') || 'HEAD';
    const separator = core.getInput('separator') || ' ';
    
    // Determine base ref dynamically if not provided
    if (!baseRef) {
      const isPullRequest = process.env.GITHUB_EVENT_NAME === 'pull_request';
      if (isPullRequest) {
        const baseBranch = process.env.GITHUB_BASE_REF;
        if (!baseBranch) {
          throw new Error('GITHUB_BASE_REF is not set for the pull request.');
        }
        baseRef = `origin/${baseBranch}`;
      } else {
        baseRef = 'HEAD^';
      }
    }
    // Parse files to check
    const filePatterns = files.split('\n').filter(pattern => pattern.trim() !== '');
    
    // Get all changed files between the refs
    let changedFilesOutput = '';
    await exec('git', ['diff', '--name-only', `${baseRef}...${headRef}`], {
      listeners: {
        stdout: (data: Buffer) => {
          changedFilesOutput += data.toString();
        }
      }
    });
    
    // Process changed files
    const allChangedFiles = changedFilesOutput.trim().split('\n').filter(file => file.trim() !== '');
    
    // Check which of the specified patterns have changed files
    const matchedFiles = new Set<string>();
    
    for (const pattern of filePatterns) {
      // For exact file paths
      if (allChangedFiles.includes(pattern)) {
        matchedFiles.add(pattern);
        continue;
      }
      
      // For glob patterns, check using minimatch
      let grepOutput = '';
      try {
        await exec('git', ['diff', '--name-only', `${baseRef}...${headRef}`, '--', pattern], {
          listeners: {
            stdout: (data: Buffer) => {
              grepOutput += data.toString();
            }
          }
        });
        
        const matches = grepOutput.trim().split('\n').filter(file => file.trim() !== '');
        matches.forEach(match => matchedFiles.add(match));
      } catch (error) {
        core.debug(`Failed to check pattern ${pattern}: ${error}`);
      }
    }
    
    // Prepare outputs
    const matchedFilesArray = Array.from(matchedFiles);
    const filesChanged = matchedFilesArray.length > 0;
    const allFilesChanged = filePatterns.length > 0 && matchedFilesArray.length >= filePatterns.length;
    
    // Set outputs
    core.setOutput('changed_files', matchedFilesArray.join(separator));
    core.setOutput('files_changed', filesChanged.toString());
    core.setOutput('all_files_changed', allFilesChanged.toString());
    
    // Log results
    core.info(`Changed files: ${matchedFilesArray.join(', ') || 'none'}`);
    core.info(`Files changed: ${filesChanged}`);
    core.info(`All files changed: ${allFilesChanged}`);
    
  } catch (error) {
    if (error instanceof Error) {
      core.setFailed(error.message);
    } else {
      core.setFailed('An unknown error occurred');
    }
  }
}

// Only call run() if executed directly
if (require.main === module) {
  run().finally(() => {
    process.exit();
  });
}
