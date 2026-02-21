# ELK Stack Setup for Stellar Insights

## Overview
Centralized logging using Elasticsearch, Logstash, and Kibana (ELK) stack.

## Quick Start

### 1. Start ELK Stack
```bash
docker-compose -f docker-compose.elk.yml up -d
```

### 2. Verify Services
- Elasticsearch: http://localhost:9200
- Kibana: http://localhost:5601
- Logstash: http://localhost:9600

### 3. Configure Backend
Add to `.env`:
```bash
LOGSTASH_HOST=localhost:5000
RUST_LOG=info
```

### 4. View Logs in Kibana
1. Open http://localhost:5601
2. Go to Management → Stack Management → Index Patterns
3. Create index pattern: `stellar-insights-*`
4. Go to Analytics → Discover to view logs

## Architecture

```
Backend (Rust) → Logstash (TCP:5000) → Elasticsearch → Kibana
```

## Log Format
JSON structured logs with fields:
- `timestamp`: ISO8601 timestamp
- `log_level`: info/warn/error
- `message`: Log message
- `service`: stellar-insights
- `target`: Rust module path

## Index Management

Logs are stored in daily indices: `stellar-insights-YYYY.MM.dd`

### Retention Policy (Optional)
```bash
# Delete indices older than 30 days
curl -X DELETE "localhost:9200/stellar-insights-*" \
  -H 'Content-Type: application/json' \
  -d '{"max_age": "30d"}'
```

## Monitoring

### Check Elasticsearch Health
```bash
curl http://localhost:9200/_cluster/health?pretty
```

### Check Logstash Pipeline
```bash
curl http://localhost:9600/_node/stats/pipelines?pretty
```

## Production Considerations

1. **Security**: Enable X-Pack security in production
2. **Scaling**: Use Elasticsearch cluster for high volume
3. **Retention**: Configure ILM policies
4. **Backup**: Regular snapshots of Elasticsearch data
5. **Resources**: Adjust JVM heap sizes based on load

## Troubleshooting

### No logs appearing
- Check Logstash is running: `docker logs stellar-logstash`
- Verify backend can connect to port 5000
- Check Elasticsearch indices: `curl localhost:9200/_cat/indices`

### High memory usage
- Reduce ES_JAVA_OPTS in docker-compose.elk.yml
- Implement log retention policy

## Stop ELK Stack
```bash
docker-compose -f docker-compose.elk.yml down
```

## Remove All Data
```bash
docker-compose -f docker-compose.elk.yml down -v
```
