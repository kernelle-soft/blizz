#!/usr/bin/env python3
"""
LanceDB Database Inspector GUI

Install dependencies:
pip install lancedb streamlit pandas

Usage:
streamlit run lancedb_inspector.py
"""
# violet ignore file

import streamlit as st
import pandas as pd
import os
import glob
from pathlib import Path

# Page configuration
st.set_page_config(
    page_title="LanceDB Inspector",
    page_icon="🔍",
    layout="wide"
)

st.title("🔍 LanceDB Database Inspector")

# Database path input
st.sidebar.header("Database Connection")

# Default path based on our Blizz application
home_dir = os.path.expanduser("~")
default_path = os.path.join(home_dir, ".blizz", "persistent", "insights", "lancedb")
db_path = st.sidebar.text_input(
    "Database Path",
    value=default_path,
    help="Path to your LanceDB database directory"
)

# Auto-discover LanceDB files
if os.path.exists(db_path):
    handle_db_exists(db_path)
else:
    handle_db_missing(db_path)


def discover_lance_files(db_path):
    """Discover LanceDB files in the given path."""
    return glob.glob(os.path.join(db_path, "**", "*.lance"), recursive=True)


def create_table_selector(lance_files):
    """Create table selection UI element."""
    return st.sidebar.selectbox(
        "Select Table",
        options=lance_files,
        format_func=lambda x: os.path.basename(x).replace('.lance', '')
    )


def display_table_metrics(table, name, path):
    """Display table metrics in columns."""
    col1, col2, col3 = st.columns(3)
    with col1:
        count = table.count_rows()
        st.metric("Total Records", count)
    with col2:
        st.metric("Table Name", name)
    with col3:
        st.metric("Database Path", path)


def display_schema_info(table):
    """Display table schema information."""
    st.header("📋 Table Schema")
    schema_info = [
        {"Field": field_name, "Type": str(field_type)}
        for field_name, field_type in table.schema().items()
    ]
    st.dataframe(pd.DataFrame(schema_info))


def display_sample_records(table):
    """Display sample records from table."""
    st.header("📊 Sample Records")
    sample_df = table.limit(10).to_pandas()
    st.dataframe(sample_df)
    return sample_df


def handle_search_ui(df):
    """Handle search UI and logic."""
    st.header("🔍 Search Records")
    col = st.selectbox("Search in column", df.columns.tolist())
    term = st.text_input("Search term")

    if term:
        filtered = df[df[col].astype(str).str.contains(term, case=False, na=False)]
        st.write(f"Found {len(filtered)} matching records")
        st.dataframe(filtered)


def handle_raw_query(db):
    """Handle raw query functionality."""
    st.header("⚡ Raw Query")
    query = st.text_area("LanceDB Query", "SELECT * FROM table LIMIT 100")
    if st.button("Execute Query"):
        try:
            result_df = db.query(query).to_pandas()
            st.dataframe(result_df)
        except Exception as e:
            st.error(f"Query failed: {e}")


def connect_to_database(db_path, selected_file):
    """Connect to database and display all info."""
    try:
        import lancedb
        
        db = lancedb.connect(db_path)
        name = os.path.basename(selected_file).replace('.lance', '')
        st.success(f"Connected to table: {name}")

        table = db.open_table(name)
        
        display_table_metrics(table, name, db_path)
        display_schema_info(table)
        df = display_sample_records(table)
        handle_search_ui(df)
        handle_raw_query(db)
        
    except ImportError as e:
        st.error(f"Missing dependencies: {e}")
        st.info("Install with: pip install lancedb streamlit pandas")
    except Exception as e:
        st.error(f"Connection failed: {e}")


def handle_db_exists(db_path):
    """Handle when database path exists."""
    files = discover_lance_files(db_path)
    st.sidebar.success(f"Found {len(files)} LanceDB files")

    if files:
        selected = create_table_selector(files)
        if st.sidebar.button("🔗 Connect to Database"):
            connect_to_database(db_path, selected)


def handle_db_missing(db_path):
    """Handle when database path is missing."""
    st.sidebar.error(f"Database path not found: {db_path}")
    st.sidebar.info("Make sure your LanceDB database exists at this location")

# Instructions
st.sidebar.header("📖 Instructions")
st.sidebar.markdown("""
1. Install dependencies:
   ```bash
   pip install lancedb streamlit pandas
   ```

2. Set your database path above

3. Select a table from the dropdown

4. Click "Connect to Database"

5. Explore your data!
""")

# Footer
st.markdown("---")
st.markdown("*LanceDB Inspector - Built for debugging and data exploration*")
