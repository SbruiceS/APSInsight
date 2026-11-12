# frozen_string_literal: true

module Api
  module V1
    class AgricudaController < ApplicationController
      def capabilities
        render json: AgricudaAdapter.new.capabilities
      end

      def run
        source = params.require(:source)
        adapter = AgricudaAdapter.new

        result = if adapter.available?
                   adapter.run(source)
                 else
                   adapter.run_double(source)
                 end

        AuditEvent.create!(
          action: "agricuda_run",
          resource_type: "AgriCUDa",
          metadata: {
            available: adapter.available?,
            source_length: source.length,
            status: result[:status]
          }
        )

        render json: result
      rescue AgricudaAdapter::Unavailable, AgricudaAdapter::ExecutionError => e
        render json: { error: e.message, capabilities: adapter.capabilities }, status: :service_unavailable
      end

      def check
        source = params.require(:source)
        adapter = AgricudaAdapter.new
        result = adapter.available? ? adapter.check(source) : { ok: false, note: "binary unavailable" }
        render json: result
      end
    end
  end
end
