# frozen_string_literal: true

module Api
  module V1
    class AgriqlController < ApplicationController
      def capabilities
        render json: AgriqlAdapter.new.capabilities
      end

      def query
        sql = params.require(:sql)
        dataset_id = params[:dataset_id]
        project_id = params[:project_id]

        run = QueryRun.create!(
          sql: sql,
          dataset_id: dataset_id,
          project_id: project_id,
          status: "running",
          provenance: {
            engine: "AgriQL",
            version: "3.10.1",
            requested_at: Time.current.iso8601
          }
        )

        adapter = AgriqlAdapter.new
        begin
          result = if adapter.available?
                     adapter.query(sql)
                   else
                     adapter.query_double(sql)
                   end

          run.update!(
            status: "succeeded",
            result: result,
            provenance: run.provenance.merge(
              completed_at: Time.current.iso8601,
              available: adapter.available?,
              row_count: result[:row_count]
            )
          )

          AuditEvent.create!(
            action: "agriql_query",
            resource_type: "QueryRun",
            resource_id: run.id,
            metadata: { sql_preview: sql[0..200], row_count: result[:row_count] }
          )

          render json: { query_run_id: run.id, result: result }
        rescue AgriqlAdapter::Unavailable, AgriqlAdapter::QueryError => e
          run.update!(status: "failed", error_message: e.message)
          render json: { error: e.message, query_run_id: run.id }, status: :unprocessable_entity
        end
      end

      def schema_discovery
        adapter = AgriqlAdapter.new
        sql = "SELECT table_schema, table_name, column_name, data_type FROM information_schema.columns"
        result = adapter.available? ? adapter.query(sql) : adapter.query_double(sql)
        render json: result
      end
    end
  end
end
