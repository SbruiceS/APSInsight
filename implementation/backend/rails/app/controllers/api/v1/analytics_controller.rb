# frozen_string_literal: true

module Api
  module V1
    class AnalyticsController < ApplicationController
      def soil_summary
        dataset = Dataset.find(params[:dataset_id])
        result = SoilAnalyticsService.new(dataset).summary
        AuditEvent.create!(
          action: "analytics_soil_summary",
          resource_type: "Dataset",
          resource_id: dataset.id,
          metadata: { methodology: result[:methodology] }
        )
        render json: result
      end
    end
  end
end
