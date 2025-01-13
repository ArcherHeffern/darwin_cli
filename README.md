# Usage 
The head TA is the only person who needs to know how to use this program. This tool takes student submissions and the project skeleton as input, runs tests + scan for plagiarism and create HTML reports containing students source code (syntax highlighted), and their test results. Distribute these reports to your TA's for them to grade.

## Generated Report
![Demo](https://i.giphy.com/media/v1.Y2lkPTc5MGI3NjExZTd3NTFqZXJ6ZmxidHF3enoxZmkxaTRiNGdzcWtqa2dnZzEwemdociZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/R9k6iSHWqMY7GVIqAG/giphy.gif)

## Installation
https://crates.io/crates/darwin_cli  
`cargo install darwin_cli`  

Ensure your install worked by running `darwin_cli`. This should show the Darwin help message.  

## Quick Use
Download _all student submissions_ zipfile from moodle

Download _project sleleton_. This should have the classic maven project structure:

```vermatim 
skel
| -- pom.xml
| -- src
      | -- main
      |     | -- (main files)
      |
      | -- test
            | -- (test files)
```

Run `darwin_cli grade PROJECT_SKELETON MOODLE_SUBMISSIONS_ZIPFILE`

You will be prompted on which tests to run

Your reports will be in the report/ directory and the plagiarism report will be at plagiarism.html

View each by opening plagiarism.html or report/index.html in chrome.   

Share the reports to your TA's and assign each TA one to grade. 

## Advanced CLI

### 1: Create Darwin Project

Run `darwin_cli create-project PROJECT_SKELETON MOODLE_SUBMISSIONS_ZIPFILE` : Initialize darwin  

This will create a .darwin folder containing copies of all submissions as diffs and the skeleton code. You are free to delete the source and submission folders now.  

### 2: Check for plagiarism
`darwin_cli plagiarism-check dest.html`

### 3: Run tests
Figure out which tests are available using `darwin_cli list-tests`  

Then run a test using `darwin_cli test-all TEST [NUM_THREADS]`. I had the best results using 4 threads.

### 4: Create Report
`darwin_cli create-report DEST-PATH NUM-PARTS [TESTS]`

This will create your report at dest path split into N parts, including test results from the tests listed. 

## Commands
create-project                           
delete-project                           
list-students                            
list-tests                               
view-student-submission                  
test-student                             
test-all                                 
view-student-result-summary              
view-student-result-by-class-name        
view-student-results-verbose             
view-all-students-results-summary        
view-all-students-results-by-class-name  
download-results-summary                 
download-results-by-class-name           
create-report                            
plagiarism-check                         
plagiarism-check-students                
anonomize                                
clean 

# Stats
## Memory Saving (Using PA1)
- All students submissions Zipped: 75M
- All student submissions Unzipped: 308M
- All student Diffs using autograder: 2MB

__Memory reduction: 99.35%__

## Grading Speed increase
Previous its uncertain how much faster this program is, since grading was previously fully manual (Installation of submissions, exporting into Eclipse, running tests, etc). Rest assured, it's significantly faster to use this tool.

### 1 thread
real    4m52.224s
user    9m53.217s
sys     0m35.208s

### 4 threads
real    2m51.790s
user    16m56.524s
sys     1m5.665s

### 8 threads
real    2m50.506s
user    16m51.381s
sys     1m10.835s

# Adding New Project Grading Types
In progress! See `src/project_runner/mod.rs` and `src/project_runner/maven.rs`.

# Developer Documentation
## darwin.json Config File
In progress!
```verbatim
{
      version: string,
      project_type: string,
      tests: [],
      tests_run: [],
      extraction_errors: {
            student: reason
      }
}
```

## .darwin folder structure
```verbatim
.darwin
| -- diff_exclude/
|     | -- code that students can't override (eg. testfiles). Gets symlinked into normalized projects
| 
| -- projects/  
|     | -- ${student_name}
|     |     | -- (normalized project patch code)
|     |    
|     | -- ...
|
| -- results/  
|     | -- ${student_name}_{test name}  
|
| -- skel/
|     | -- (normalized project source code)
|
| -- submission_diffs/  
|     | -- ${student_name}  
|
| -- compile_errors
```     

### Example Maven Project Structure
```verbatim
.darwin
| -- diff_exclude
|     | -- src/test/
|               | -- ...
|     
| -- skel
|     | -- pom.xml (Patched version)  
|     | -- src/main/
|               | -- ...
| 
| -- projects
|     | -- student1
|           | -- src/
|           |     | -- main/
|           |     | -- test -> .darwin/diff_exclude/src/test
|           |
|           | -- pom.xml
|
| -- ...
```

## Report Folder Structure
```verbatim
report
| -- index.html (Contains student list)
| -- tests.html (Contains all tests with syntax highlighting, will be iframed into all student java files)
|
| -- students/
|     | -- $(student_name)
|                 | -- $(filename).java
|                 | -- pom.xml
|
| -- styles/
```

## Plagiarism Detection
- Detection: Locality Sensitive hash using TLSH
- Visualising: https://en.wikipedia.org/wiki/Multidimensional_scaling aka Principal Coordinates Analysis (PCoA)

# Error File Formats
```verbatim
FILE = LINE*
LINE = STUDENT_NAME ":" ERROR_REASON "\n"
STUDENT_NAME = [^\n:]+
ERROR_REASON = [^\n]*
```
