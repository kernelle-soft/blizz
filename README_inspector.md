# LanceDB Database Inspector Tools

Two tools to inspect your LanceDB database outside of the main application.

## 🖥️ Option 1: Streamlit Web GUI (Recommended)

**Best for:** Visual browsing, searching, and exploring data

### Setup:
```bash
pip install lancedb streamlit pandas
```

### Usage:
```bash
cd /home/jeff/code/blizz
streamlit run lancedb_inspector.py
```

Then open: http://localhost:8501

### Features:
- ✅ Web interface at `http://localhost:8501`
- ✅ Auto-detects your Blizz database at `~/.blizz/persistent/insights/lancedb`
- ✅ Browse tables and records
- ✅ Search functionality  
- ✅ Schema inspection
- ✅ Sample data preview

---

## 🖥️ Option 2: Rust CLI Tool 

**Best for:** Quick inspection, no external dependencies

### Usage:
```bash
cd /home/jeff/code/blizz
cargo run --bin inspect_lancedb -p insights
```

### Features:
- ✅ No external dependencies needed
- ✅ Shows table info, record count, sample records
- ✅ Works immediately

### Sample Output:
```
🔍 LanceDB Database Inspector
============================
Database path: /home/jeff/.blizz/persistent/insights/lancedb

📋 Tables found: ["insights_embeddings"]

📊 Inspecting 'insights_embeddings' table...
Total embeddings: 15

📝 Sample records:

--- Record 1 ---
  ID: cursor-basics:getting-started
  Topic: cursor-basics
  Name: getting-started
  Overview: Basic introduction to using Cursor editor...

--- Record 2 ---
  ID: rust-patterns:error-handling
  Topic: rust-patterns  
  Name: error-handling
  Overview: Best practices for error handling in Rust...
```

---

## Database Location

Your LanceDB database is stored at:
```
~/.blizz/persistent/insights/lancedb/
├── insights_embeddings.lance  # Your vector embeddings
└── _versions/                 # Version metadata
```

## Troubleshooting

**"Database directory does not exist"**
- Run the insights server first: `insights_server`
- Add some insights to create the database

**"No tables found"** 
- Add insights using `insights add` or the REST API
- Run `insights index` to populate the vector database

**Python dependencies missing**
- Install with: `pip install lancedb streamlit pandas`
