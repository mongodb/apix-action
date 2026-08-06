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
    '^@actions/core$': '<rootDir>/src/__tests__/mocks/actions-core.cjs',
    '^@actions/exec$': '<rootDir>/src/__tests__/mocks/actions-exec.cjs'
  },
  moduleFileExtensions: ['ts', 'js', 'json', 'node']
};
