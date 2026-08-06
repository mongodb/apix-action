module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'node',
  verbose: true,
  clearMocks: true,
  testMatch: ['**/*.test.ts'],
  transform: {
    '^.+\\.ts$': ['ts-jest', { tsconfig: { module: 'CommonJS', moduleResolution: 'Node' } }]
  },
  moduleNameMapper: {
    '^@actions/exec$': '<rootDir>/node_modules/@actions/core/node_modules/@actions/exec/lib/exec.js'
  },
  moduleFileExtensions: ['ts', 'js', 'json', 'node']
};
