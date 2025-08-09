#!/usr/bin/env python3
"""
Script to fix compilation errors in the multi-user system implementation.
This script will systematically fix missing user_id fields in Task and ConversationEntry structures.
"""

import re
import os

def fix_task_structures(file_path):
    """Fix missing user_id fields in Task structures."""
    with open(file_path, 'r') as f:
        content = f.read()
    
    # Pattern to match Task { id: row.get("id"), case_id: ... without user_id
    pattern = r'(tasks\.push\(Task \{\s*id: row\.get\("id"\),\s*)(case_id:)'
    replacement = r'\1user_id: row.get("user_id"),\n                \2'
    
    content = re.sub(pattern, replacement, content)
    
    # Also fix direct Task construction
    pattern2 = r'(Ok\(Task \{\s*id: row\.get\("id"\),\s*)(case_id:)'
    replacement2 = r'\1user_id: row.get("user_id"),\n            \2'
    
    content = re.sub(pattern2, replacement2, content)
    
    with open(file_path, 'w') as f:
        f.write(content)
    
    print(f"Fixed Task structures in {file_path}")

def fix_conversation_entries(file_path):
    """Fix missing user_id fields in ConversationEntry structures."""
    with open(file_path, 'r') as f:
        content = f.read()
    
    # Pattern to match ConversationEntry { id: row.get("id"), case_id: ... without user_id
    pattern = r'(entries\.push\(ConversationEntry \{\s*id: row\.get\("id"\),\s*)(case_id:)'
    replacement = r'\1user_id: row.get("user_id"),\n                \2'
    
    content = re.sub(pattern, replacement, content)
    
    with open(file_path, 'w') as f:
        f.write(content)
    
    print(f"Fixed ConversationEntry structures in {file_path}")

def main():
    # Fix the main database working file
    db_file = "/Users/ravikadam/Documents/1learning/tasks/services/persistence-service/src/database_working.rs"
    
    if os.path.exists(db_file):
        fix_task_structures(db_file)
        fix_conversation_entries(db_file)
        print("Compilation fixes applied successfully!")
    else:
        print(f"Database file not found: {db_file}")

if __name__ == "__main__":
    main()
