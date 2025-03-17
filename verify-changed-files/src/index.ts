import * as core from '@actions/core';
import { exec } from '@actions/exec';

// Export the run function for testing
export async function run(): Promise<void> {
  try {
    // Get inputs
    const files = core.getInput('files', { required: true });
    const baseRef = core.getInput('base-ref') || 'HEAD^';
    const headRef = core.getInput('head-ref') || 'HEAD';
    const separator = core.getInput('separator') || ' ';
    
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
    const anyChanged = matchedFilesArray.length > 0;
    const allChanged = filePatterns.length > 0 && matchedFilesArray.length >= filePatterns.length;
    
    // Set outputs
    core.setOutput('changed_files', matchedFilesArray.join(separator));
    core.setOutput('any_changed', anyChanged.toString());
    core.setOutput('all_changed', allChanged.toString());
    
    // Log results
    core.info(`Changed files: ${matchedFilesArray.join(', ') || 'none'}`);
    core.info(`Any files changed: ${anyChanged}`);
    core.info(`All files changed: ${allChanged}`);
    
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
  run();
}
