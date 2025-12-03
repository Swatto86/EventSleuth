#!/bin/bash

# EventSleuth Test Runner Script (Bash)
# This script runs all tests (frontend and backend) and generates reports

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Test results
frontend_passed=false
backend_passed=false
start_time=$(date +%s)

# Function to print section headers
print_section() {
    echo ""
    echo -e "${YELLOW}======================================${NC}"
    echo -e "${YELLOW} $1${NC}"
    echo -e "${YELLOW}======================================${NC}"
    echo ""
}

# Function to print success message
print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

# Function to print error message
print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Function to print info message
print_info() {
    echo -e "${BLUE}ℹ $1${NC}"
}

echo -e "${CYAN}======================================${NC}"
echo -e "${CYAN}   EventSleuth Test Suite Runner     ${NC}"
echo -e "${CYAN}======================================${NC}"
echo ""

# Navigate to script directory
cd "$(dirname "$0")"

print_info "Project directory: $(pwd)"
echo ""

# ===========================================
# FRONTEND TESTS
# ===========================================
print_section "Running Frontend Tests (Vitest)"

print_info "Installing/checking dependencies..."
npm install --silent

if [ $? -eq 0 ]; then
    print_info "Running frontend test suite..."
    echo ""

    if npm test -- --run --reporter=verbose; then
        print_success "Frontend tests passed!"
        frontend_passed=true
    else
        print_error "Frontend tests failed!"
    fi
else
    print_error "Failed to install dependencies"
fi

# ===========================================
# FRONTEND COVERAGE
# ===========================================
print_section "Generating Frontend Coverage Report"

print_info "Running tests with coverage..."
npm run test:coverage -- --run > /dev/null 2>&1

if [ -f "coverage/index.html" ]; then
    print_success "Coverage report generated: coverage/index.html"

    # Parse coverage summary if available
    if [ -f "coverage/coverage-summary.json" ]; then
        echo ""
        echo -e "${CYAN}Coverage Summary:${NC}"

        # Extract coverage percentages using grep and awk
        if command -v jq &> /dev/null; then
            lines=$(jq -r '.total.lines.pct' coverage/coverage-summary.json)
            statements=$(jq -r '.total.statements.pct' coverage/coverage-summary.json)
            functions=$(jq -r '.total.functions.pct' coverage/coverage-summary.json)
            branches=$(jq -r '.total.branches.pct' coverage/coverage-summary.json)

            echo "  Lines:      ${lines}%"
            echo "  Statements: ${statements}%"
            echo "  Functions:  ${functions}%"
            echo "  Branches:   ${branches}%"
        else
            print_info "Install jq for detailed coverage stats: sudo apt-get install jq"
        fi
    fi
else
    print_error "Coverage report not generated"
fi

# ===========================================
# BACKEND TESTS
# ===========================================
print_section "Running Backend Tests (Rust/Cargo)"

cd src-tauri

print_info "Checking Rust toolchain..."
rust_version=$(cargo --version)
print_info "Using: $rust_version"

print_info "Running backend test suite..."
echo ""

if cargo test --color=always; then
    print_success "Backend tests passed!"
    backend_passed=true
else
    print_error "Backend tests failed!"
fi

cd ..

# ===========================================
# BACKEND TESTS (DETAILED)
# ===========================================
print_section "Running Backend Tests (Verbose Output)"

cd src-tauri

print_info "Running tests with detailed output..."
echo ""

cargo test -- --nocapture --test-threads=1

cd ..

# ===========================================
# SUMMARY
# ===========================================
print_section "Test Summary"

end_time=$(date +%s)
duration=$((end_time - start_time))

echo -e "${CYAN}Execution Time: ${duration} seconds${NC}"
echo ""

if [ "$frontend_passed" = true ]; then
    print_success "Frontend Tests: PASSED"
else
    print_error "Frontend Tests: FAILED"
fi

if [ "$backend_passed" = true ]; then
    print_success "Backend Tests: PASSED"
else
    print_error "Backend Tests: FAILED"
fi

echo ""

if [ "$frontend_passed" = true ] && [ "$backend_passed" = true ]; then
    echo -e "${GREEN}======================================${NC}"
    echo -e "${GREEN}   ALL TESTS PASSED! ✓               ${NC}"
    echo -e "${GREEN}======================================${NC}"

    # Ask to open coverage report
    read -p "Open coverage report in browser? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        if [ -f "coverage/index.html" ]; then
            if command -v xdg-open &> /dev/null; then
                xdg-open coverage/index.html
            elif command -v open &> /dev/null; then
                open coverage/index.html
            else
                print_info "Coverage report: $(pwd)/coverage/index.html"
            fi
        fi
    fi

    exit 0
else
    echo -e "${RED}======================================${NC}"
    echo -e "${RED}   SOME TESTS FAILED ✗               ${NC}"
    echo -e "${RED}======================================${NC}"

    exit 1
fi
