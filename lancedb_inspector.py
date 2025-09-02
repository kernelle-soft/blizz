#!/usr/bin/env python3
"""
LanceDB Database Inspector GUI

Install dependencies:
pip install lancedb streamlit pandas

Usage:
streamlit run lancedb_inspector.py
"""

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
    lance_files = glob.glob(os.path.join(db_path, "**", "*.lance"), recursive=True)
    st.sidebar.success(f"Found {len(lance_files)} LanceDB files")

    if lance_files:
        selected_file = st.sidebar.selectbox(
            "Select Table",
            options=lance_files,
            format_func=lambda x: os.path.basename(x).replace('.lance', '')
        )

        if st.sidebar.button("🔗 Connect to Database"):
            try:
                import lancedb

                # Connect to database
                db = lancedb.connect(db_path)

                # Get table name from file
                table_name = os.path.basename(selected_file).replace('.lance', '')

                st.success(f"Connected to table: {table_name}")

                # Get table
                table = db.open_table(table_name)

                # Show table info
                col1, col2, col3 = st.columns(3)
                with col1:
                    count = table.count_rows()
                    st.metric("Total Records", count)
                with col2:
                    st.metric("Table Name", table_name)
                with col3:
                    st.metric("Database Path", db_path)

                # Schema info
                st.header("📋 Table Schema")
                schema_info = []
                for field_name, field_type in table.schema().items():
                    schema_info.append({
                        "Field": field_name,
                        "Type": str(field_type)
                    })
                st.dataframe(pd.DataFrame(schema_info))

                # Sample data
                st.header("📊 Sample Records")
                sample_df = table.limit(10).to_pandas()
                st.dataframe(sample_df)

                # Search functionality
                st.header("🔍 Search Records")
                search_col = st.selectbox("Search in column", sample_df.columns.tolist())
                search_term = st.text_input("Search term")

                if search_term:
                    # Simple text search
                    filtered_df = sample_df[
                        sample_df[search_col].astype(str).str.contains(search_term, case=False, na=False)
                    ]
                    st.write(f"Found {len(filtered_df)} matching records")
                    st.dataframe(filtered_df)

                # Raw SQL queries (if supported)
                st.header("⚡ Raw Query")
                query = st.text_area("LanceDB Query", "SELECT * FROM table LIMIT 100")
                if st.button("Execute Query"):
                    try:
                        result_df = db.query(query).to_pandas()
                        st.dataframe(result_df)
                    except Exception as e:
                        st.error(f"Query failed: {e}")

            except ImportError as e:
                st.error(f"Missing dependencies: {e}")
                st.info("Install with: pip install lancedb streamlit pandas")
            except Exception as e:
                st.error(f"Connection failed: {e}")

else:
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
