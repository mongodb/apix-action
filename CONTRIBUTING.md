# Contributing to APIX Actions

Thank you for your interest in contributing to APIX Actions! This document provides guidelines and instructions for contributing.

## Code of Conduct

Please be respectful and considerate of others when contributing to this project.

## How to Contribute

1. **Fork the repository** and create your branch from `main`.
2. **Make your changes** and ensure they follow the project's coding style.
3. **Test your changes** thoroughly.
4. **Submit a pull request** with a clear description of the changes and any relevant issue numbers.

## Development Setup

1. Clone the repository:
   ```
   git clone https://github.com/your-org/apix-action.git
   cd apix-action
   ```

2. For actions with Node.js components:
   ```
   cd action-directory
   npm install
   npm run build
   npm test
   ```

## Creating New Actions

When creating a new action:

1. Follow the existing directory structure and naming conventions
2. Include a clear README.md that describes:
   - Purpose of the action
   - Required inputs
   - Optional inputs
   - Outputs
   - Example usage

3. Write tests for your action
4. Build the action if necessary (for JavaScript/TypeScript actions)
5. Update the root README.md to list your new action

## Testing

- For JavaScript/TypeScript actions, run `npm test` within the action directory
- For composite actions, test the workflow using the GitHub Actions workflow in your fork

## Release Process

Releases are created automatically when pushing a tag that starts with `v`:

```
git tag v1.0.0
git push origin v1.0.0
```

## Questions?

If you have any questions, feel free to open an issue.

## License

By contributing, you agree that your contributions will be licensed under the project's [Apache License 2.0](LICENSE).