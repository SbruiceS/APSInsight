# frozen_string_literal: true

module Api
  module V1
    class FarmsController < ApplicationController
      def index
        farms = Farm.includes(:organization, :project).order(:name)
        render json: farms.as_json(include: [:organization, :project])
      end

      def show
        farm = Farm.find(params[:id])
        render json: farm.as_json(include: [:organization, :project, :fields])
      end

      def create
        farm = Farm.new(farm_params)
        if farm.save
          AuditEvent.create!(action: "create", resource_type: "Farm", resource_id: farm.id)
          render json: farm, status: :created
        else
          render json: { errors: farm.errors.full_messages }, status: :unprocessable_entity
        end
      end

      def update
        farm = Farm.find(params[:id])
        if farm.update(farm_params)
          render json: farm
        else
          render json: { errors: farm.errors.full_messages }, status: :unprocessable_entity
        end
      end

      private

      def farm_params
        params.require(:farm).permit(:organization_id, :project_id, :name, :slug, :description, :area_hectares, :location)
      end
    end
  end
end
