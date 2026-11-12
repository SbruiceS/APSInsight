# frozen_string_literal: true

module Api
  module V1
    class DataQualityReportsController < ApplicationController
      def index
        reports = DataQualityReport.includes(:dataset, :dataset_version, :import).order(created_at: :desc)
        render json: reports.as_json(include: [:dataset, :dataset_version, :import])
      end

      def show
        render json: DataQualityReport.find(params[:id]).as_json(include: [:dataset, :dataset_version, :import])
      end

      def create
        report = DataQualityReport.new(data_quality_report_params)
        if report.save
          AuditEvent.create!(action: "create", resource_type: "DataQualityReport", resource_id: report.id)
          render json: report, status: :created
        else
          render json: { errors: report.errors.full_messages }, status: :unprocessable_entity
        end
      end

      private

      def data_quality_report_params
        params.require(:data_quality_report).permit(
          :dataset_id, :dataset_version_id, :import_id, :status,
          :row_count, :null_count, summary: {}, issues: []
        )
      end
    end
  end
end
